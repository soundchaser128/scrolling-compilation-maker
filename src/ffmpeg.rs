use std::{fmt, path::Path, process::Stdio, sync::LazyLock, time::Duration};

use color_eyre::Result;
use color_eyre::eyre::bail;
use indicatif::{FormattedDuration, ProgressBar, ProgressState, ProgressStyle};
use regex::Regex;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::Command,
};
use tracing::{debug, info};

use crate::types::{ClipInfo, EncodingArgs};

fn ffmpeg_progress_bar(total_duration_ms: u64) -> ProgressBar {
    let style = ProgressStyle::with_template(
        "{msg} {elapsed} {wide_bar:.cyan/blue} Encoded {pos_duration} / {len_duration}, ETA: {eta}",
    )
    .unwrap()
    .with_key(
        "pos_duration",
        |state: &ProgressState, w: &mut dyn fmt::Write| {
            write!(
                w,
                "{}",
                FormattedDuration(Duration::from_millis(state.pos()))
            )
            .unwrap()
        },
    )
    .with_key(
        "len_duration",
        |state: &ProgressState, w: &mut dyn fmt::Write| {
            write!(
                w,
                "{}",
                FormattedDuration(Duration::from_millis(state.len().unwrap()))
            )
            .unwrap()
        },
    );
    let pb = ProgressBar::new(total_duration_ms)
        .with_style(style)
        .with_message("Creating compilation");
    if crate::progress_hidden() {
        ProgressBar::hidden()
    } else {
        pb
    }
}

pub async fn create_scrolling_video(
    clips: &[ClipInfo],
    output: &str,
    viewport_width: u32,
    viewport_height: u32,
    duration_secs: u32,
    encoding: &EncodingArgs,
    text: Option<Text>,
    audio_path: Option<&Path>,
) -> Result<()> {
    static OUT_TIME_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"out_time_us=(\d+)").unwrap());

    let total_width: u32 = clips.iter().map(|c| c.output_width).sum();
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
        text,
    );
    info!("Filter graph:\n{filter_graph}");

    let mut cmd = Command::new("ffmpeg");
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    for clip in clips {
        if clip.is_image {
            // -loop 1 makes ffmpeg generate a continuous video stream from a still image
            cmd.arg("-loop").arg("1");
        } else {
            cmd.arg("-stream_loop").arg("-1");
        }
        cmd.arg("-i").arg(&clip.path);
    }

    // Audio input is added after all video inputs
    let audio_input_index = clips.len();
    if let Some(audio) = audio_path {
        cmd.arg("-i").arg(audio);
    }

    cmd.arg("-filter_complex").arg(&filter_graph);
    cmd.arg("-map").arg("[out]");
    if audio_path.is_some() {
        cmd.arg("-map").arg(format!("{audio_input_index}:a"));
        cmd.arg("-c:a").arg("aac");
        cmd.arg("-shortest");
    } else {
        cmd.arg("-an");
    }
    cmd.arg("-t").arg(duration_secs.to_string());
    cmd.arg("-c:v").arg(&encoding.codec);
    for arg in &encoding.preset_args {
        cmd.arg(arg);
    }
    cmd.arg(encoding.quality_flag)
        .arg(encoding.quality_value.to_string());
    cmd.arg("-y");
    cmd.arg("-progress");
    cmd.arg("-");
    cmd.arg("-nostats");
    cmd.arg(output);

    info!("Running ffmpeg...");
    let mut process = cmd.spawn()?;

    let stdout = process.stdout.take().unwrap();
    let reader = BufReader::new(stdout);

    let mut last_position = 0;
    let progress = ffmpeg_progress_bar((duration_secs * 1000) as u64);
    let mut lines = reader.lines();
    while let Some(line) = lines.next_line().await? {
        debug!("{}", line);
        if let Some(captures) = OUT_TIME_REGEX.captures(&line) {
            let duration: u64 = captures.get(1).unwrap().as_str().parse::<u64>()?;
            let duration = Duration::from_micros(duration);
            let millis = duration.as_millis() as u64;
            let delta = millis - last_position;
            progress.inc(delta);
            last_position = millis;
        }
    }
    progress.finish_and_clear();
    let exit_code = process.wait().await?;
    if !exit_code.success() {
        let reader = process.stderr.take().unwrap();
        let mut reader = BufReader::new(reader);
        let mut stderr = String::new();
        reader.read_to_string(&mut stderr).await?;
        bail!("ffmpeg failed with error code {exit_code}:\n{stderr}");
    }

    info!("Output written to {output}");
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum Text {
    Performers,
    Tags,
}

fn build_filter_graph(
    clips: &[ClipInfo],
    viewport_width: u32,
    viewport_height: u32,
    speed: f64,
    max_offset: u32,
    text: Option<Text>,
) -> String {
    let n = clips.len();
    let mut parts = Vec::new();

    // Scale each input to the target height, optionally crop to aspect ratio,
    // with optional performer name overlay
    for (i, clip) in clips.iter().enumerate() {
        let mut chain = format!(
            "[{i}:v]fps=30,scale={w}:{h},setpts=PTS-STARTPTS",
            w = clip.scaled_width,
            h = viewport_height,
        );
        // Center-crop wider clips to the target aspect ratio
        if clip.output_width < clip.scaled_width {
            let cw = clip.output_width;
            chain.push_str(&format!(",crop={cw}:{viewport_height}:(iw-{cw})/2:0"));
        }
        if let Some(text) = text {
            let text = match text {
                Text::Performers => clip.performers.join(", "),
                Text::Tags => clip.tags.join(", "),
            };
            let text = escape_drawtext(&text);
            chain.push_str(&format!(
                ",drawtext=text='{text}':fontsize=24:fontcolor=white:\
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

pub async fn check_ffmpeg() -> Result<()> {
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
