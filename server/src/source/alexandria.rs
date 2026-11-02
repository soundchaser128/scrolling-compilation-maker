use std::sync::Arc;

use color_eyre::{Result, eyre::bail};
use futures::stream::{self, StreamExt};
use reqwest::{Client, Url};
use serde::Deserialize;
use tracing::warn;

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
    async fn fetch_pool(&self, params: PoolParams) -> Result<Vec<MediaFile>> {
        let mut candidates: Vec<MediaFile> = Vec::new();
        let mut page = 0u32;
        let seed = params.seed.to_string();

        loop {
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
                for p in &params.people {
                    q.append_pair("person", p);
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
                candidates.push(m);
            }

            page += 1;
            // Gather 3x the pool size to survive reachability filtering
            if page >= total_pages || candidates.len() >= params.pool_size * 3 {
                break;
            }
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
        warn!("Returned {} of {} requested", pool.len(), params.pool_size);
        Ok(pool)
    }

    async fn fetch_people(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        let mut url = Url::parse(&format!("{}/api/people/autocomplete", self.api_url))?;
        if let Some(prefix) = prefix {
            url.query_pairs_mut().append_pair("query", prefix);
        }
        let people: Vec<PersonResponse> = self.client.get(url).send().await?.json().await?;
        Ok(people.into_iter().map(|p| p.name).collect())
    }

    async fn fetch_tags(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        let mut url = Url::parse(&format!("{}/api/tag", self.api_url))?;
        if let Some(prefix) = prefix {
            url.query_pairs_mut().append_pair("query", prefix);
        }
        let tags: Vec<TagCompletion> = self.client.get(url).send().await?.json().await?;
        Ok(tags.into_iter().map(|t| t.name).collect())
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
    count: i64,
}
