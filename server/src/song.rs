use color_eyre::Result;
use std::path::{Path, PathBuf};

/// Download audio (blocking) off the async runtime.
pub async fn download_song(url: &str, dest_dir: &Path) -> Result<PathBuf> {
    let url = url.to_string();
    let dest = dest_dir.to_path_buf();
    tokio::task::spawn_blocking(move || download_song_blocking(&url, &dest))
        .await
        .map_err(|e| color_eyre::eyre::eyre!("song download task panicked: {e}"))?
}

fn download_song_blocking(url: &str, dest_dir: &Path) -> Result<PathBuf> {
    let status = std::process::Command::new("yt-dlp")
        .args(["-x", "--audio-format", "m4a", "-o"])
        .arg(dest_dir.join("song.%(ext)s"))
        .arg(url)
        .status()?;
    if !status.success() {
        color_eyre::eyre::bail!("yt-dlp failed: {status}");
    }
    let path = dest_dir.join("song.m4a");
    if !path.exists() {
        color_eyre::eyre::bail!("yt-dlp did not produce audio");
    }
    Ok(path)
}
