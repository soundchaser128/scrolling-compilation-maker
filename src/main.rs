mod api;
mod download;
mod ffmpeg;
mod song;
mod types;

use clap::Parser;
use color_eyre::Result;
use reqwest::Client;
use tracing::{info, level_filters::LevelFilter};

use std::time::Duration;

use crate::{
    api::FetchVideosParams,
    types::{
        AspectRatio, ClipInfo, Codec, Effort, EncodingArgs, Orientation, Quality,
        generate_output_name, parse_duration,
    },
};

#[derive(Parser)]
#[command(name = "scrolling-compilation-maker")]
#[command(about = "Create scrolling video compilations from portrait video clips")]
struct Args {
    /// Number of video clips to include
    #[arg(short = 'n', long, default_value_t = 20)]
    clip_count: usize,

    /// Output file path (derived from filters or randomly generated if omitted)
    #[arg(short, long)]
    output: Option<String>,

    /// Output video width (viewport)
    #[arg(long, default_value_t = 1920)]
    width: u32,

    /// Output video height (viewport)
    #[arg(long, default_value_t = 1080)]
    height: u32,

    /// Total output video duration (e.g. "60", "1m", "1m30s")
    #[arg(short, long, default_value = "1m", value_parser = parse_duration)]
    duration: Duration,

    /// Maximum duration per source clip (e.g. "30s", "1m")
    #[arg(long, default_value = "30s", value_parser = parse_duration)]
    max_clip_duration: Duration,

    /// Random seed for consistent API ordering (random if not set)
    #[arg(long)]
    seed: Option<f64>,

    /// Filter by orientation (portrait, landscape, square, any). Omit for portrait.
    #[arg(long, value_enum, default_value = "portrait")]
    orientation: Orientation,

    /// Filter by tag (can be specified multiple times)
    #[arg(long = "tag")]
    tags: Vec<String>,

    /// Filter by person/performer name (can be specified multiple times)
    #[arg(long = "person")]
    people: Vec<String>,

    /// Include images in the compilation
    #[arg(long)]
    with_images: bool,

    /// Crop clips to this aspect ratio before combining (e.g. "9:16", "3:4").
    /// Useful when selecting landscape/square videos to make them narrower.
    #[arg(long, value_parser = AspectRatio::parse)]
    crop: Option<AspectRatio>,

    /// API base URL
    #[arg(long, default_value = "https://alexandria.soundchaser128.com")]
    api_url: String,

    /// Content CDN base URL
    #[arg(long, default_value = "https://content.r2.soundchaser128.com")]
    content_url: String,

    /// API authentication token
    #[arg(long, env = "ALEXANDRIA_API_TOKEN")]
    api_token: Option<String>,

    /// Number of concurrent downloads
    #[arg(long, default_value_t = 4)]
    download_concurrency: usize,

    /// Output codec
    #[arg(long, value_enum, default_value_t = Codec::X264)]
    codec: Codec,

    /// Output quality
    #[arg(long, value_enum, default_value_t = Quality::Medium)]
    quality: Quality,

    /// Encoding effort (higher = slower but better compression)
    #[arg(long, value_enum, default_value_t = Effort::Medium)]
    effort: Effort,

    /// Use GPU-accelerated encoding (NVIDIA NVENC)
    #[arg(long)]
    gpu: bool,

    /// URL of a song to use as audio. Downloaded with yt-dlp; sets compilation
    /// duration to the song's length.
    #[arg(long)]
    song: Option<String>,

    /// Enable logging with the specified level.
    #[arg(long)]
    log: Option<LevelFilter>,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    let seed = args.seed.unwrap_or_else(|| rand::random::<f64>());
    let output = args
        .output
        .unwrap_or_else(|| generate_output_name(&args.tags, &args.people, &args.orientation));
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

    // 3. Compute clip info with scaled dimensions
    let crop_width = args.crop.as_ref().map(|a| a.crop_width(args.height));
    let clips: Vec<ClipInfo> = videos
        .iter()
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
            let performer_name = v
                .people
                .iter()
                .find(|p| p.person_type == "performer")
                .map(|p| p.name.clone());
            ClipInfo {
                path: p.clone(),
                scaled_width: scaled_w,
                output_width: output_w,
                performer_name,
                is_image: v.is_image(),
            }
        })
        .collect();

    // 4. Create scrolling video
    let encoding = EncodingArgs::new(&args.codec, &args.quality, &args.effort, args.gpu);
    ffmpeg::create_scrolling_video(
        &clips,
        &output,
        args.width,
        args.height,
        duration.as_secs() as u32,
        &encoding,
        args.people.is_empty(),
        audio_path.as_deref(),
    )
    .await?;

    Ok(())
}
