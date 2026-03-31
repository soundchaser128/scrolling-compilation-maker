use std::{path::PathBuf, process::Stdio, time::Duration};

use color_eyre::Result;
use color_eyre::eyre::bail;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::process::Command;
use tracing::info;

/// Download audio from a URL using yt-dlp, returning the path to the downloaded file.
pub async fn download_song(url: &str, dest_dir: &std::path::Path) -> Result<PathBuf> {
    check_yt_dlp().await?;

    let output_template = dest_dir.join("song.%(ext)s");
    let spinner = ProgressBar::new_spinner()
        .with_style(ProgressStyle::with_template("{spinner} {msg}").unwrap());
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner.set_message(format!("Downloading audio from {url}"));

    let status = Command::new("yt-dlp")
        .arg("-x") // extract audio
        .arg("--audio-format")
        .arg("m4a")
        .arg("-o")
        .arg(&output_template)
        .arg(url)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    spinner.finish_and_clear();

    if !status.success() {
        bail!("yt-dlp failed to download audio from '{url}'");
    }

    let path = dest_dir.join("song.m4a");
    if !path.exists() {
        bail!(
            "yt-dlp did not produce expected output file at {}",
            path.display()
        );
    }

    info!("Downloaded song to {}", path.display());
    Ok(path)
}

/// Get the duration of an audio file using ffprobe.
pub async fn probe_duration(path: &std::path::Path) -> Result<Duration> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("quiet")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(path)
        .output()
        .await?;

    if !output.status.success() {
        bail!("ffprobe failed on {}", path.display());
    }

    let text = String::from_utf8(output.stdout)?;
    let secs: f64 = text.trim().parse().map_err(|_| {
        color_eyre::eyre::eyre!(
            "could not parse duration from ffprobe output: '{}'",
            text.trim()
        )
    })?;

    Ok(Duration::from_secs_f64(secs))
}

async fn check_yt_dlp() -> Result<()> {
    let output = Command::new("yt-dlp")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;

    match output {
        Ok(status) if status.success() => Ok(()),
        _ => bail!("yt-dlp not found. Please install yt-dlp and ensure it's in your PATH."),
    }
}
