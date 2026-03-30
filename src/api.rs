use color_eyre::eyre::{self, bail};
use reqwest::{Client, Url};
use tracing::{info, warn};

use crate::types::{PageResponse, VideoFile};

pub async fn fetch_videos(
    client: &Client,
    api_url: &str,
    max_clip_duration_ms: u64,
    desired_count: usize,
    api_token: Option<&str>,
    seed: f64,
    tags: &[String],
    people: &[String],
) -> eyre::Result<Vec<VideoFile>> {
    let mut videos = Vec::new();
    let mut page = 0u32;
    let seed = seed.to_string();

    loop {
        let page_str = page.to_string();
        let mut url = Url::parse(api_url).unwrap();
        url.set_path("/api/file");
        let mut query = vec![
            ("orientation", "Portrait"),
            ("sort", "random"),
            ("fileType", "video"),
            ("size", "50"),
            ("page", &page_str),
            ("seed", &seed),
        ];
        for person in people {
            query.push(("person", &person));
        }
        for tag in tags {
            query.push(("tag", tag));
        }

        for (k, v) in query {
            url.query_pairs_mut().append_pair(k, v);
        }

        let mut request = client.get(url);
        if let Some(token) = api_token {
            request = request.header("Cookie", format!("SESSION={token}"));
        }

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

            if video.duration.unwrap() > max_clip_duration_ms {
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

    warn!(
        "Only found {} clips out of {} requested",
        videos.len(),
        desired_count
    );
    Ok(videos)
}
