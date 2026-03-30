use std::process::Stdio;

use color_eyre::eyre::{self, bail};
use tokio::process::Command;
use tracing::info;

use crate::types::ClipInfo;

pub async fn create_scrolling_video(
    clips: &[ClipInfo],
    output: &str,
    viewport_width: u32,
    viewport_height: u32,
    duration_secs: u32,
    codec: &str,
    crf: u32,
    show_performer_names: bool,
) -> eyre::Result<()> {
    let total_width: u32 = clips.iter().map(|c| c.scaled_width).sum();
    if total_width <= viewport_width {
        bail!(
            "Total canvas width ({total_width}) must be greater than viewport width ({viewport_width}). \
             Add more clips or use a narrower viewport."
        );
    }

    let max_offset = total_width - viewport_width;
    let speed = max_offset as f64 / duration_secs as f64;

    let filter_graph = build_filter_graph(
        clips,
        viewport_width,
        viewport_height,
        speed,
        max_offset,
        show_performer_names,
    );
    info!("Filter graph:\n{filter_graph}");

    let mut cmd = Command::new("ffmpeg");

    for clip in clips {
        if clip.is_image {
            // -loop 1 makes ffmpeg generate a continuous video stream from a still image
            cmd.arg("-loop").arg("1");
        } else {
            cmd.arg("-stream_loop").arg("-1");
        }
        cmd.arg("-i").arg(&clip.path);
    }

    cmd.arg("-filter_complex").arg(&filter_graph);
    cmd.arg("-map").arg("[out]");
    cmd.arg("-t").arg(duration_secs.to_string());
    cmd.arg("-c:v").arg(codec);
    cmd.arg("-crf").arg(crf.to_string());
    cmd.arg("-an");
    cmd.arg("-y");
    cmd.arg(output);

    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    info!("Running ffmpeg...");
    let status = cmd.status().await?;

    if !status.success() {
        bail!("ffmpeg exited with status {status}");
    }

    info!("Output written to {output}");
    Ok(())
}

fn build_filter_graph(
    clips: &[ClipInfo],
    viewport_width: u32,
    viewport_height: u32,
    speed: f64,
    max_offset: u32,
    show_performer_names: bool,
) -> String {
    let n = clips.len();
    let mut parts = Vec::new();

    // Scale each input to the target height, with optional performer name overlay
    for (i, clip) in clips.iter().enumerate() {
        let mut chain = format!(
            "[{i}:v]fps=30,scale={w}:{h},setpts=PTS-STARTPTS",
            w = clip.scaled_width,
            h = viewport_height,
        );
        if let (true, Some(name)) = (show_performer_names, &clip.performer_name) {
            let escaped = escape_drawtext(name);
            chain.push_str(&format!(
                ",drawtext=text='{escaped}':fontsize=24:fontcolor=white:\
                 borderw=2:bordercolor=black:\
                 x=(w-text_w)/2:y=h-text_h-20"
            ));
        }
        chain.push_str(&format!("[v{i}]"));
        parts.push(chain);
    }

    // Stack all videos horizontally
    let inputs: String = (0..n).map(|i| format!("[v{i}]")).collect();
    parts.push(format!("{inputs}hstack=inputs={n}[canvas]"));

    // Crop with scrolling offset
    parts.push(format!(
        "[canvas]crop={viewport_width}:{viewport_height}:'min(t*{speed:.2},{max_offset})':0[out]"
    ));

    parts.join(";\n")
}

/// Escape special characters for ffmpeg's drawtext filter.
/// Colons, backslashes, and single quotes need escaping.
fn escape_drawtext(text: &str) -> String {
    text.replace('\\', "\\\\\\\\")
        .replace(':', "\\:")
        .replace('\'', "'\\\\\\''")
}

pub async fn check_ffmpeg() -> eyre::Result<()> {
    let output = Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;

    match output {
        Ok(status) if status.success() => Ok(()),
        _ => bail!("ffmpeg not found. Please install ffmpeg and ensure it's in your PATH."),
    }
}
