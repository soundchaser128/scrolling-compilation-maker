mod api;
mod download;
mod ffmpeg;
mod song;
mod types;

use clap::Parser;
use color_eyre::Result;
use reqwest::Client;
use tracing::{info, level_filters::LevelFilter};

use std::{cmp::Ordering, sync::atomic::AtomicBool, time::Duration};

use crate::{
    api::FetchVideosParams,
    ffmpeg::Text,
    types::{
        AspectRatio, ClipInfo, Codec, Effort, EncodingArgs, Orientation, Quality, ScrollEasing,
        generate_output_name, parse_duration,
    },
};

static PROGRESS_HIDDEN: AtomicBool = AtomicBool::new(false);

pub fn progress_hidden() -> bool {
    PROGRESS_HIDDEN.load(std::sync::atomic::Ordering::SeqCst)
}

#[derive(Parser)]
#[command(name = "scrolling-compilation-maker")]
#[command(about = "Create scrolling video compilations from portrait video clips")]
pub struct Args {
    /// Number of video clips to include
    #[arg(short = 'n', long, default_value_t = 20)]
    pub clip_count: usize,

    /// Output file path (derived from filters or randomly generated if omitted)
    #[arg(short, long)]
    pub output: Option<String>,

    /// Output video width (viewport)
    #[arg(long, default_value_t = 1920)]
    pub width: u32,

    /// Output video height (viewport)
    #[arg(long, default_value_t = 1080)]
    pub height: u32,

    /// Total output video duration (e.g. "60", "1m", "1m30s")
    #[arg(short, long, default_value = "1m", value_parser = parse_duration)]
    pub duration: Duration,

    /// Text to render with each clip.
    #[arg(long)]
    pub text: Option<Text>,

    /// Maximum duration per source clip (e.g. "30s", "1m")
    #[arg(long, default_value = "30s", value_parser = parse_duration)]
    pub max_clip_duration: Duration,

    /// Random seed for consistent API ordering (random if not set)
    #[arg(long)]
    pub seed: Option<f64>,

    /// Filter by orientation (portrait, landscape, square, any). Omit for portrait.
    #[arg(long, value_enum, default_value = "portrait")]
    pub orientation: Orientation,

    /// Filter by tag (can be specified multiple times)
    #[arg(long = "tag")]
    pub tags: Vec<String>,

    /// Filter by person/performer name (can be specified multiple times)
    #[arg(long = "person")]
    pub people: Vec<String>,

    /// Include images in the compilation
    #[arg(long)]
    pub with_images: bool,

    /// Crop clips to this aspect ratio before combining (e.g. "9:16", "3:4").
    /// Useful when selecting landscape/square videos to make them narrower.
    #[arg(long, value_parser = AspectRatio::parse)]
    pub crop: Option<AspectRatio>,

    /// API base URL
    #[arg(long, default_value = "https://alexandria.soundchaser128.com")]
    pub api_url: String,

    /// Content CDN base URL
    #[arg(long, default_value = "https://content.r2.soundchaser128.com")]
    pub content_url: String,

    /// Number of concurrent downloads
    #[arg(long, default_value_t = 4)]
    pub download_concurrency: usize,

    /// Output codec
    #[arg(long, value_enum, default_value_t = Codec::X264)]
    pub codec: Codec,

    /// Output quality
    #[arg(long, value_enum, default_value_t = Quality::Medium)]
    pub quality: Quality,

    /// Encoding effort (higher = slower but better compression)
    #[arg(long, value_enum, default_value_t = Effort::Medium)]
    pub effort: Effort,

    /// Use GPU-accelerated encoding (NVIDIA NVENC)
    #[arg(long)]
    pub gpu: bool,

    /// URL of a song to use as audio. Downloaded with yt-dlp; sets compilation
    /// duration to the song's length.
    #[arg(long)]
    pub song: Option<String>,

    /// Scrolling easing mode
    #[arg(long, value_enum, default_value_t = ScrollEasing::Linear)]
    pub easing: ScrollEasing,

    /// Enable logging with the specified level.
    #[arg(long)]
    pub log: Option<LevelFilter>,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    let seed = args.seed.unwrap_or_else(|| rand::random::<f64>());
    let output = args
        .output
        .unwrap_or_else(|| generate_output_name(&args.tags, &args.people));
    if let Some(filter) = args.log {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive(format!("scrolling_compilation_maker={filter}").parse()?),
            )
            .init();
    }

    info!("Using seed: {seed}");
    info!("Output file: {output}");

    ffmpeg::check_ffmpeg().await?;

    let client = Client::new();
    let temp_dir = tempfile::tempdir()?;
    let progress_hidden = args.log.is_some();

    PROGRESS_HIDDEN.store(progress_hidden, std::sync::atomic::Ordering::SeqCst);

    // 0. Download song if provided
    let mut duration = args.duration;
    let audio_path = match &args.song {
        Some(url) => {
            let path = song::download_song(url, temp_dir.path()).await?;
            let song_duration = song::probe_duration(&path).await?;
            info!("Song duration: {:.1}s", song_duration.as_secs_f64());
            duration = song_duration;
            Some(path)
        }
        None => None,
    };

    // 1. Fetch video metadata
    info!("Fetching video metadata...");
    let videos = api::fetch_videos(
        &client,
        FetchVideosParams {
            api_url: &args.api_url,
            max_clip_duration: args.max_clip_duration,
            desired_count: args.clip_count,
            seed,
            orientation: args.orientation,
            tags: &args.tags,
            people: &args.people,
            with_images: args.with_images,
        },
    )
    .await?;
    info!("Selected {} clips or images", videos.len());
    // 2. Download clips
    info!("Downloading clips...");
    let paths = download::download_clips(
        &client,
        &videos,
        &args.content_url,
        temp_dir.path(),
        args.download_concurrency,
    )
    .await?;
    info!(?paths, "Downlaoded clips");

    // 3. Compute clip info with scaled dimensions
    let crop_width = args.crop.as_ref().map(|a| a.crop_width(args.height));
    let mut clips: Vec<ClipInfo> = videos
        .into_iter()
        .zip(paths.iter())
        .map(|(v, p)| {
            let w = v.width.unwrap() as u32;
            let h = v.height.unwrap() as u32;
            let mut scaled_w = (w as u64 * args.height as u64 / h as u64) as u32;
            // Round up to even (required by most codecs)
            scaled_w += scaled_w % 2;
            // If cropping, cap output width (only affects wider clips)
            let output_w = match crop_width {
                Some(cw) => scaled_w.min(cw),
                None => scaled_w,
            };
            ClipInfo {
                is_image: v.is_image(),
                path: p.clone(),
                scaled_width: scaled_w,
                output_width: output_w,
                performers: v.people.into_iter().map(|p| p.name).collect(),
                tags: v.tags,
                popularity: v.popularity,
            }
        })
        .collect();

    clips.sort_by(|a, b| {
        a.popularity
            .partial_cmp(&b.popularity)
            .unwrap_or(Ordering::Equal)
    });

    // 4. Create scrolling video
    let encoding = EncodingArgs::new(&args.codec, &args.quality, &args.effort, args.gpu);
    ffmpeg::create_scrolling_video(
        &clips,
        &output,
        args.width,
        args.height,
        duration.as_secs() as u32,
        &encoding,
        args.text,
        audio_path.as_deref(),
        &args.easing,
    )
    .await?;

    println!("Compilation generated to {output}");

    Ok(())
}
