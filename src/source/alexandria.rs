use std::sync::Arc;

use color_eyre::Result;
use color_eyre::eyre::bail;
use reqwest::{Url, blocking::Client};
use serde::Deserialize;
use tracing::{info, warn};

use crate::{
    source::{FetchVideosParams, MediaSource},
    types::{MediaFile, PageResponse},
};

#[derive(Clone)]
pub struct AlexandriaMediaSource {
    api_url: Arc<String>,
    content_url: Arc<String>,
    client: Client,
}

impl AlexandriaMediaSource {
    pub fn new(api_url: String, content_url: String) -> Self {
        AlexandriaMediaSource {
            api_url: Arc::new(api_url),
            content_url: Arc::new(content_url),
            client: Client::new(),
        }
    }

    fn media_reachable(&self, file: &MediaFile, base_url: &str) -> Result<bool> {
        let response = self.client.head(file.content_url(base_url)).send()?;
        Ok(response.status().is_success())
    }
}

impl MediaSource for AlexandriaMediaSource {
    fn fetch(
        &self,
        FetchVideosParams {
            max_clip_duration,
            desired_count,
            seed,
            orientation,
            tags,
            people,
            with_images,
        }: FetchVideosParams<'_>,
    ) -> Result<Vec<MediaFile>> {
        let mut media = Vec::new();
        let mut page = 0u32;
        let seed = seed.to_string();

        loop {
            let page_str = page.to_string();
            assert!(
                self.api_url.len() > 0,
                "API URL '{}' must not be empty",
                self.api_url
            );
            let mut url = Url::parse(&self.api_url).unwrap();
            url.set_path("/api/file");
            let mut query = vec![
                ("sort", "random"),
                ("fileType", "video"),
                ("size", "50"),
                ("page", &page_str),
                ("seed", &seed),
                ("withTags", "true"),
            ];

            if with_images {
                query.push(("fileType", "image"));
            }
            if let Some(o) = orientation.as_api_param() {
                query.push(("orientation", o));
            }
            for person in people {
                query.push(("person", &person));
            }
            for tag in tags {
                query.push(("tag", tag));
            }

            for (k, v) in query {
                url.query_pairs_mut().append_pair(k, v);
            }
            let request = self.client.get(url);
            let response = request.send()?;
            if !response.status().is_success() {
                bail!(
                    "API request failed with status {}: {}",
                    response.status(),
                    response.text().unwrap_or_default()
                );
            }

            let page_response: PageResponse<MediaFile> = response.json()?;
            let total_pages = page_response.page.total_pages;

            for media_item in page_response.content {
                let dominated = media_item.width.is_none()
                    || media_item.height.is_none()
                    || media_item.duration.is_none();
                if dominated {
                    continue;
                }

                if media_item.duration.unwrap() > max_clip_duration.as_millis() as u64 {
                    continue;
                }

                if !self
                    .media_reachable(&media_item, &self.content_url)
                    .unwrap_or(false)
                {
                    continue;
                }

                media.push(media_item);
                if media.len() >= desired_count {
                    info!("Collected {} clips", media.len());
                    return Ok(media);
                }
            }

            page += 1;
            if page >= total_pages {
                break;
            }
        }

        if media.is_empty() {
            bail!("No suitable videos found");
        }
        warn!(
            "Only found {} clips out of {} requested",
            media.len(),
            desired_count
        );
        Ok(media)
    }

    fn fetch_people(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        let mut url = Url::parse(&format!("{}/api/people/autocomplete", self.api_url))?;
        if let Some(prefix) = prefix {
            url.query_pairs_mut().append_pair("query", prefix);
        }
        let people: Vec<PersonResponse> = self.client.get(url).send()?.json()?;
        Ok(people.into_iter().map(|p| p.name).collect())
    }

    fn fetch_tags(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        todo!()
    }
}

#[derive(Deserialize)]
struct PersonResponse {
    id: i64,
    name: String,
}
