use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
};

use color_eyre::{Result, eyre::bail};
use futures::stream::{self, StreamExt};
use reqwest::{Client, Url};
use serde::Deserialize;

use crate::{
    source::{MediaSource, PoolParams},
    types::{MediaFile, PageResponse},
};

#[derive(Clone)]
pub struct AlexandriaMediaSource {
    api_url: Arc<str>,
    content_url: Arc<str>,
    client: Client,
}

impl AlexandriaMediaSource {
    pub fn new(api_url: String, content_url: String) -> Self {
        Self {
            api_url: api_url.into(),
            content_url: content_url.into(),
            client: Client::new(),
        }
    }

    async fn reachable(&self, file: &MediaFile) -> bool {
        let Ok(resp) = self
            .client
            .head(file.content_url(&self.content_url))
            .send()
            .await
        else {
            return false;
        };
        resp.status().is_success()
    }
}

#[allow(async_fn_in_trait)]
impl MediaSource for AlexandriaMediaSource {
    #[tracing::instrument(skip_all, fields(pool_size = params.pool_size, max_clip_duration_ms = params.max_clip_duration_ms, with_images = params.with_images), err)]
    async fn fetch_pool(&self, params: PoolParams) -> Result<Vec<MediaFile>> {
        let mut candidates: Vec<MediaFile> = Vec::new();
        let mut seen_files = HashSet::new();
        let mut seen_people = HashSet::new();
        let mut queries: Vec<_> = params
            .people
            .iter()
            .filter(|person| seen_people.insert(person.as_str()))
            .map(|person| (Some(person.as_str()), 0u32))
            .collect();
        if queries.is_empty() {
            queries.push((None, 0));
        }
        let seed = params.seed.to_string();

        while !queries.is_empty() && candidates.len() < params.pool_size * 3 {
            let mut next_queries = Vec::new();
            let mut pages = Vec::new();
            for (person, page) in queries {
                let mut url = Url::parse(&self.api_url)?;
                url.set_path("/api/file");
                {
                    let mut q = url.query_pairs_mut();
                    q.append_pair("sort", "random");
                    q.append_pair("fileType", "video");
                    q.append_pair("size", "50");
                    q.append_pair("page", &page.to_string());
                    q.append_pair("seed", &seed);
                    q.append_pair("withTags", "true");
                    if params.with_images {
                        q.append_pair("fileType", "image");
                    }
                    if let Some(o) = params.orientation.as_api_param() {
                        q.append_pair("orientation", o);
                    }
                    if let Some(person) = person {
                        q.append_pair("person", person);
                    }
                    for t in &params.tags {
                        q.append_pair("tag", t);
                    }
                }

                let resp = self.client.get(url).send().await?;
                if !resp.status().is_success() {
                    bail!(
                        "API request failed with status {}: {}",
                        resp.status(),
                        resp.text().await.unwrap_or_default()
                    );
                }
                let page_response: PageResponse<MediaFile> = resp.json().await?;
                let total_pages = page_response.page.total_pages;
                tracing::debug!(
                    page,
                    person,
                    total_pages,
                    count = page_response.content.len(),
                    "fetched media page"
                );

                let mut media = VecDeque::new();
                for m in page_response.content {
                    if m.width.is_none()
                        || m.height.is_none()
                        || (!m.is_image() && m.duration.is_none())
                    {
                        continue; // missing dimensions/duration
                    }
                    if m.duration.unwrap_or(0) > params.max_clip_duration_ms {
                        continue;
                    }
                    media.push_back(m);
                }

                pages.push(media);
                if page + 1 < total_pages {
                    next_queries.push((person, page + 1));
                }
            }

            // Interleave each person's results so the pool limit does not favor
            // the first selected person. Shared files appear only once.
            while pages.iter().any(|page| !page.is_empty()) {
                for page in &mut pages {
                    if let Some(media) = page.pop_front() {
                        if seen_files.insert(media.id) {
                            candidates.push(media);
                        }
                    }
                }
            }
            queries = next_queries;
        }

        let src = self.clone();
        let pool: Vec<MediaFile> = stream::iter(candidates.into_iter())
            .map(|m| {
                let src = src.clone();
                async move {
                    if src.reachable(&m).await {
                        Some(m)
                    } else {
                        None
                    }
                }
            })
            .buffer_unordered(8)
            .filter_map(|m| async move { m })
            .take(params.pool_size)
            .collect()
            .await;

        if pool.is_empty() {
            bail!("No suitable media found");
        }
        if pool.len() < params.pool_size {
            tracing::warn!(
                returned = pool.len(),
                requested = params.pool_size,
                "media pool is smaller than requested"
            );
        } else {
            tracing::debug!(returned = pool.len(), "media pool fetched");
        }
        Ok(pool)
    }

    #[tracing::instrument(skip_all, err)]
    async fn fetch_people(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        let mut url = Url::parse(&format!("{}/api/people/autocomplete", self.api_url))?;
        if let Some(prefix) = prefix {
            url.query_pairs_mut().append_pair("query", prefix);
        }
        let people: Vec<PersonResponse> = self.client.get(url).send().await?.json().await?;
        Ok(people.into_iter().map(|p| p.name).collect())
    }

    #[tracing::instrument(skip_all, err)]
    async fn fetch_tags(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        let mut url = Url::parse(&format!("{}/api/tag", self.api_url))?;
        if let Some(prefix) = prefix {
            url.query_pairs_mut().append_pair("query", prefix);
        }
        let tags: Vec<TagCompletion> = self.client.get(url).send().await?.json().await?;
        Ok(tags.into_iter().map(|t| t.full_name()).collect())
    }
}

#[derive(Deserialize)]
struct PersonResponse {
    name: String,
}

#[derive(Deserialize)]
struct TagCompletion {
    name: String,
    category: Option<String>,
}

impl TagCompletion {
    fn full_name(&self) -> String {
        if let Some(category) = &self.category {
            format!("{}:{}", category, self.name)
        } else {
            self.name.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Orientation;
    use axum::{
        Json, Router,
        extract::{RawQuery, State},
        routing::get,
    };
    use serde_json::json;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    type Requests = Arc<Mutex<Vec<Vec<(String, String)>>>>;

    async fn files(
        State(requests): State<Requests>,
        RawQuery(query): RawQuery,
    ) -> Json<serde_json::Value> {
        let url = Url::parse(&format!("http://localhost/?{}", query.unwrap_or_default())).unwrap();
        let pairs: Vec<_> = url.query_pairs().into_owned().collect();
        let people: Vec<_> = pairs
            .iter()
            .filter(|(key, _)| key == "person")
            .map(|(_, value)| value.as_str())
            .collect();
        assert!(
            people.len() <= 1,
            "each request must filter by at most one person"
        );
        let page = pairs
            .iter()
            .find(|(key, _)| key == "page")
            .unwrap()
            .1
            .parse::<u32>()
            .unwrap();
        let ids = match (people.first().copied(), page) {
            (Some("Alice"), 0) => vec![1, 2],
            (Some("Bob"), 0) => vec![2],
            (Some("Bob"), 1) => vec![3],
            (None, 0) => vec![4],
            _ => panic!("unexpected request: {pairs:?}"),
        };
        let total_pages = if people == ["Bob"] { 2 } else { 1 };
        requests.lock().await.push(pairs);
        Json(json!({
            "content": ids.into_iter().map(|id| json!({
                "id": Uuid::from_u128(id), "fileType": "video", "width": 640,
                "height": 480, "duration": 1000, "mimeType": "video/mp4",
                "popularity": 0.0, "tags": []
            })).collect::<Vec<_>>(),
            "page": {"size": 50, "number": page, "totalElements": 3, "totalPages": total_pages}
        }))
    }

    #[tokio::test]
    async fn fetches_people_separately_with_pagination_and_deduplication() {
        let requests = Requests::default();
        let app = Router::new()
            .route("/api/file", get(files))
            .fallback(|| async { axum::http::StatusCode::OK })
            .with_state(requests.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let source = AlexandriaMediaSource::new(base_url.clone(), base_url);

        for (people, expected_ids, expected_requests) in [
            (vec!["Alice", "Bob", "Alice"], vec![1, 2, 3], 3),
            (vec!["Alice"], vec![1, 2], 1),
            (vec![], vec![4], 1),
        ] {
            requests.lock().await.clear();
            let pool = source
                .fetch_pool(PoolParams {
                    pool_size: 10,
                    max_clip_duration_ms: 30_000,
                    seed: 0.5,
                    orientation: Orientation::Portrait,
                    tags: vec!["tag".into()],
                    people: people.into_iter().map(String::from).collect(),
                    with_images: true,
                })
                .await
                .unwrap();
            let ids: HashSet<_> = pool.iter().map(|file| file.id).collect();
            assert_eq!(pool.len(), expected_ids.len());
            assert_eq!(ids, expected_ids.into_iter().map(Uuid::from_u128).collect());
            let recorded = requests.lock().await;
            assert_eq!(recorded.len(), expected_requests);
            for query in recorded.iter() {
                for (key, value) in [
                    ("seed", "0.5"),
                    ("tag", "tag"),
                    ("orientation", "Portrait"),
                    ("fileType", "video"),
                    ("fileType", "image"),
                ] {
                    assert!(query.contains(&(key.into(), value.into())));
                }
            }
        }
        server.abort();
    }
}
