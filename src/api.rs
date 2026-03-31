use std::{cmp::Ordering, time::Duration};

use color_eyre::Result;
use color_eyre::eyre::bail;
use reqwest::{Client, Url};
use tracing::{info, warn};

use crate::types::{Orientation, PageResponse, VideoFile};

pub struct FetchVideosParams<'a> {
    pub api_url: &'a str,
    pub max_clip_duration: Duration,
    pub desired_count: usize,
    pub seed: f64,
    pub orientation: Orientation,
    pub tags: &'a [String],
    pub people: &'a [String],
    pub with_images: bool,
}

pub async fn fetch_videos(
    client: &Client,
    FetchVideosParams {
        api_url,
        max_clip_duration,
        desired_count,
        seed,
        orientation,
        tags,
        people,
        with_images,
    }: FetchVideosParams<'_>,
) -> Result<Vec<VideoFile>> {
    let mut videos = Vec::new();
    let mut page = 0u32;
    let seed = seed.to_string();

    loop {
        let page_str = page.to_string();
        let mut url = Url::parse(api_url).unwrap();
        url.set_path("/api/file");
        let mut query = vec![
            ("sort", "random"),
            ("fileType", "video"),
            ("size", "50"),
            ("page", &page_str),
            ("seed", &seed),
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
        let request = client.get(url);
        let response = request.send().await?;
        if !response.status().is_success() {
            bail!(
                "API request failed with status {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            );
        }

        let page_response: PageResponse<VideoFile> = response.json().await?;
        let total_pages = page_response.page.total_pages;

        for video in page_response.content {
            let dominated =
                video.width.is_none() || video.height.is_none() || video.duration.is_none();
            if dominated {
                continue;
            }

            if video.duration.unwrap() > max_clip_duration.as_millis() as u64 {
                continue;
            }

            videos.push(video);
            if videos.len() >= desired_count {
                info!("Collected {} clips", videos.len());
                return Ok(videos);
            }
        }

        page += 1;
        if page >= total_pages {
            break;
        }
    }

    if videos.is_empty() {
        bail!("No suitable videos found");
    }
    videos.sort_by(|a, b| {
        a.popularity
            .partial_cmp(&b.popularity)
            .unwrap_or(Ordering::Equal)
    });

    warn!(
        "Only found {} clips out of {} requested",
        videos.len(),
        desired_count
    );
    Ok(videos)
}
