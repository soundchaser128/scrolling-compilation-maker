use crate::types::{ClipInfo, EncodingArgs, ScrollEasing};
use color_eyre::{Result, eyre::bail};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Text {
    Performers,
    Tags,
}

/// Invert the same blended smoothstep used by the scrolling crop.
fn time_at_offset(offset: f64, max_offset: u32, duration: u32, easing: ScrollEasing) -> f64 {
    if max_offset == 0 {
        return 0.0;
    }
    let target = (offset / max_offset as f64).clamp(0.0, 1.0);
    let progress = match easing {
        ScrollEasing::Linear => target,
        ScrollEasing::Ease => {
            let (mut low, mut high) = (0.0, 1.0);
            for _ in 0..50 {
                let mid = (low + high) / 2.0;
                let value = 0.3 * mid + 0.7 * mid * mid * (3.0 - 2.0 * mid);
                if value < target {
                    low = mid;
                } else {
                    high = mid;
                }
            }
            (low + high) / 2.0
        }
    };
    progress * duration as f64
}

fn build_filter_graph(
    clips: &[ClipInfo],
    viewport_width: u32,
    viewport_height: u32,
    duration_secs: u32,
    max_offset: u32,
    text: Option<Text>,
    easing: ScrollEasing,
) -> String {
    let n = clips.len();
    let mut parts = Vec::new();

    // Scale each input to the target height, optionally crop to aspect ratio,
    // with optional performer name overlay
    let mut left = 0u32;
    for (i, clip) in clips.iter().enumerate() {
        let mut chain = format!(
            "[{i}:v]fps=30,scale={w}:{h},setpts=PTS-STARTPTS",
            w = clip.scaled_width,
            h = viewport_height,
        );
        // Hold the first frame until entrance, so the selected source start is
        // the first frame seen instead of an arbitrary phase of a looping input.
        let enter = time_at_offset(
            left as f64 - viewport_width as f64,
            max_offset,
            duration_secs,
            easing,
        );
        if enter > 0.0 && !clip.is_image {
            chain.push_str(&format!(",tpad=start_mode=clone:start_duration={enter:.6}"));
        }
        left += clip.output_width;
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
    let d = duration_secs;
    let p = format!("min(t/{d},1)");
    let offset_expr = match easing {
        ScrollEasing::Linear => {
            format!("{p}*{max_offset}")
        }
        ScrollEasing::Ease => {
            // Blend of linear and smoothstep so the scroll never fully stops,
            // just slows to ~30% of max speed at start/end.
            let ease = 0.7; // 0 = fully linear, 1 = full smoothstep
            let lin = 1.0 - ease;
            let ss = format!("{p}*{p}*(3-2*{p})");
            format!("({lin}*{p}+{ease}*{ss})*{max_offset}")
        }
    };
    parts.push(format!(
        "[canvas]crop={viewport_width}:{viewport_height}:\
         'min({offset_expr},{max_offset})':0[out]"
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

#[tracing::instrument(err)]
pub fn check_ffmpeg() -> Result<()> {
    let output = Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match output {
        Ok(status) if status.success() => Ok(()),
        _ => bail!("ffmpeg not found. Please install ffmpeg and ensure it's in your PATH."),
    }
}

/// Trim a remote clip to [start_ms, end_ms] via stream copy into a local segment.
#[tracing::instrument(skip_all, fields(start_ms, end_ms), err)]
pub async fn trim_clip(url: &str, start_ms: u64, end_ms: u64, out: &Path) -> Result<()> {
    let start = format!("{:.3}", start_ms as f64 / 1000.0);
    let end = format!("{:.3}", end_ms as f64 / 1000.0);
    let output = tokio::process::Command::new("ffmpeg")
        .arg("-ss")
        .arg(&start)
        .arg("-to")
        .arg(&end)
        .arg("-i")
        .arg(url)
        .arg("-c")
        .arg("copy")
        .arg("-avoid_negative_ts")
        .arg("make_zero")
        .arg("-y")
        .arg(out)
        .output()
        .await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy_owned(output.stderr);
        bail!("ffmpeg trim failed for {url}.\nerror output: {stderr}");
    }
    Ok(())
}

pub struct RenderParams {
    pub clips: Vec<ClipInfo>,
    pub output: PathBuf,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub duration_secs: u32,
    pub encoding: EncodingArgs,
    pub text: Option<Text>,
    pub audio_path: Option<PathBuf>,
    pub easing: ScrollEasing,
    /// out_time in microseconds, updated as ffmpeg reports progress
    pub progress: Arc<AtomicU64>,
}
#[tracing::instrument(
    skip_all,
    fields(
        clip_count = params.clips.len(),
        duration_secs = params.duration_secs,
        width = params.viewport_width,
        height = params.viewport_height
    ),
    err
)]
pub async fn render(params: RenderParams) -> Result<()> {
    let RenderParams {
        clips,
        output,
        viewport_width,
        viewport_height,
        duration_secs,
        encoding,
        text,
        audio_path,
        easing,
        progress,
    } = params;

    let total_width: u32 = clips.iter().map(|c| c.output_width).sum();
    if total_width <= viewport_width {
        bail!(
            "Total canvas width ({total_width}) must exceed viewport width ({viewport_width}). \
             Select more clips or a narrower viewport."
        );
    }
    let max_offset = total_width - viewport_width;

    let filter_graph = build_filter_graph(
        &clips,
        viewport_width,
        viewport_height,
        duration_secs,
        max_offset,
        text,
        easing,
    );
    tracing::debug!("built ffmpeg filter graph");

    let mut cmd = tokio::process::Command::new("ffmpeg");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    for clip in &clips {
        if clip.is_image {
            cmd.arg("-loop").arg("1");
        } else {
            cmd.arg("-stream_loop").arg("-1");
        }
        cmd.arg("-i").arg(&clip.path);
    }

    let audio_input_index = clips.len();
    if let Some(audio) = &audio_path {
        cmd.arg("-stream_loop").arg("-1").arg("-i").arg(audio);
    }

    cmd.arg("-filter_complex").arg(&filter_graph);
    cmd.arg("-map").arg("[out]");
    if audio_path.is_some() {
        cmd.arg("-map").arg(format!("{audio_input_index}:a"));
        cmd.arg("-c:a").arg("aac");
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
    cmd.arg("-y").arg("-progress").arg("-").arg("-nostats");
    cmd.arg(&output);

    tracing::info!("starting ffmpeg render");
    let mut child = cmd.spawn()?;
    let stderr = child.stderr.take().unwrap();
    let stderr_task = tokio::spawn(async move {
        use tokio::io::AsyncReadExt;

        let mut stderr = stderr;
        let mut buf = String::new();
        let _ = stderr.read_to_string(&mut buf).await;
        buf
    });
    let stdout = child.stdout.take().unwrap();
    let mut lines = BufReader::new(stdout).lines();
    while let Some(line) = lines.next_line().await? {
        if let Some(rest) = line.strip_prefix("out_time_us=") {
            if let Ok(us) = rest.parse::<u64>() {
                progress.store(us, Ordering::Relaxed);
            }
        }
    }
    let status = child.wait().await?;
    if !status.success() {
        let buf = stderr_task.await.unwrap_or_default();
        bail!("ffmpeg failed ({status}):\n{buf}");
    }
    tracing::info!("ffmpeg render completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entrance_time_matches_scroll_position() {
        assert_eq!(time_at_offset(-100.0, 400, 60, ScrollEasing::Linear), 0.0);
        assert_eq!(time_at_offset(200.0, 400, 60, ScrollEasing::Linear), 30.0);
        assert_eq!(time_at_offset(500.0, 400, 60, ScrollEasing::Linear), 60.0);
        for fraction in [0.1, 0.25, 0.5, 0.8] {
            let t = time_at_offset(fraction * 400.0, 400, 60, ScrollEasing::Ease) / 60.0;
            let position = 0.3 * t + 0.7 * t * t * (3.0 - 2.0 * t);
            assert!((position - fraction).abs() < 1e-10);
        }
    }

    #[test]
    fn later_clips_start_playing_at_entrance() {
        let clips: Vec<_> = (0..4)
            .map(|_| ClipInfo {
                path: "clip.mp4".into(),
                scaled_width: 100,
                output_width: 100,
                performers: vec![],
                tags: vec![],
                popularity: 0.0,
                is_image: false,
            })
            .collect();
        let graph = build_filter_graph(&clips, 100, 100, 60, 300, None, ScrollEasing::Linear);
        assert!(graph.contains("tpad=start_mode=clone:start_duration=20.000000[v2]"));
        assert!(graph.contains("tpad=start_mode=clone:start_duration=40.000000[v3]"));
        assert!(!graph.contains("start_duration=0.000000"));
    }

    #[test]
    fn filter_graph_stacks_and_scrolls_selected_clips() {
        let clip = || ClipInfo {
            path: "clip.mp4".into(),
            scaled_width: 100,
            output_width: 80,
            performers: vec![],
            tags: vec![],
            popularity: 0.0,
            is_image: false,
        };
        let graph = build_filter_graph(
            &[clip(), clip()],
            64,
            128,
            60,
            96,
            None,
            ScrollEasing::Linear,
        );
        assert!(graph.contains("[v0][v1]hstack=inputs=2"));
        assert!(graph.contains("crop=80:128"));
        assert!(graph.contains("min(t/60,1)*96"));
    }
}
