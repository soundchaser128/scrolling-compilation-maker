use std::path::{Path, PathBuf};

use color_eyre::{Result, eyre::bail};
use futures::stream::{self, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::io::AsyncWriteExt;
use tracing::info;

use crate::types::{VideoFile, extension_for_mime};

fn trim_title(title: &str) -> String {
    let trimmed: String = title.chars().take(60).collect();
    trimmed.replace('\n', " ").replace('\t', " ")
}

pub async fn download_clips(
    client: &reqwest::Client,
    clips: &[VideoFile],
    content_base_url: &str,
    temp_dir: &Path,
    concurrency: usize,
) -> Result<Vec<PathBuf>> {
    let multi = MultiProgress::new();
    let style = ProgressStyle::with_template("[{bar:30}] {bytes}/{total_bytes} {msg}")
        .unwrap()
        .progress_chars("=> ");

    let paths: Vec<PathBuf> = clips
        .iter()
        .enumerate()
        .map(|(i, clip)| {
            let ext = extension_for_mime(&clip.mime_type);
            temp_dir.join(format!("clip_{i:03}{ext}"))
        })
        .collect();

    let tasks: Vec<_> = clips
        .iter()
        .zip(paths.iter())
        .map(|(clip, path)| {
            let url = clip.content_url(content_base_url);
            let path = path.clone();
            let pb = multi.add(ProgressBar::new(0));
            pb.set_style(style.clone());
            pb.set_message(format!("{}", trim_title(&clip.title)));
            (url, path, pb)
        })
        .collect();

    let results: Vec<Result<PathBuf>> = stream::iter(tasks)
        .map(|(url, path, pb)| async move {
            let response = client.get(&url).send().await?;
            if !response.status().is_success() {
                bail!("Failed to download {url}: {}", response.status());
            }

            if let Some(len) = response.content_length() {
                pb.set_length(len);
            }

            let mut file = tokio::fs::File::create(&path).await?;
            let mut stream = response.bytes_stream();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                pb.inc(chunk.len() as u64);
                file.write_all(&chunk).await?;
            }

            file.flush().await?;
            pb.finish_and_clear();
            Ok(path)
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    let mut downloaded = Vec::with_capacity(results.len());
    for result in results {
        downloaded.push(result?);
    }

    // Sort by filename to maintain original order
    downloaded.sort();

    info!("Downloaded {} clips", downloaded.len());
    Ok(downloaded)
}
