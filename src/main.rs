mod api;
mod download;
mod ffmpeg;
mod types;

use clap::Parser;
use color_eyre::Result;
use tracing::info;

use crate::types::{ClipInfo, Orientation};

#[derive(Parser)]
#[command(name = "scrolling-compilation-maker")]
#[command(about = "Create scrolling video compilations from portrait video clips")]
struct Args {
    /// Number of video clips to include
    #[arg(short = 'n', long, default_value_t = 20)]
    clip_count: usize,

    /// Output file path
    #[arg(short, long, default_value = "output.mp4")]
    output: String,

    /// Output video width (viewport)
    #[arg(long, default_value_t = 1920)]
    width: u32,

    /// Output video height (viewport)
    #[arg(long, default_value_t = 1080)]
    height: u32,

    /// Total output video duration in seconds
    #[arg(short, long, default_value_t = 60)]
    duration: u32,

    /// Maximum duration per source clip in milliseconds
    #[arg(long, default_value_t = 30000)]
    max_clip_duration_ms: u64,

    /// Random seed for consistent API ordering (random if not set)
    #[arg(long)]
    seed: Option<f64>,

    /// Filter by orientation (portrait, landscape, square). Omit for any.
    #[arg(long, value_enum)]
    orientation: Option<Orientation>,

    /// Filter by tag (can be specified multiple times)
    #[arg(long = "tag")]
    tags: Vec<String>,

    /// Filter by person/performer name (can be specified multiple times)
    #[arg(long = "person")]
    people: Vec<String>,

    /// Include images in the compilation
    #[arg(long)]
    with_images: bool,

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

    /// Video codec for output
    #[arg(long, default_value = "libx264")]
    codec: String,

    /// CRF quality value (lower = better quality, bigger file)
    #[arg(long, default_value_t = 23)]
    crf: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("scrolling_compilation_maker=info".parse()?),
        )
        .init();

    let args = Args::parse();
    let seed = args.seed.unwrap_or_else(|| rand::random::<f64>());
    info!("Using seed: {seed}");

    ffmpeg::check_ffmpeg().await?;

    let client = reqwest::Client::new();
    let temp_dir = tempfile::tempdir()?;

    // 1. Fetch video metadata
    info!("Fetching video metadata...");
    let videos = api::fetch_videos(
        &client,
        &args.api_url,
        args.max_clip_duration_ms,
        args.clip_count,
        args.api_token.as_deref(),
        seed,
        args.orientation.as_ref(),
        &args.tags,
        &args.people,
        args.with_images,
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
    let clips: Vec<ClipInfo> = videos
        .iter()
        .zip(paths.iter())
        .map(|(v, p)| {
            let w = v.width.unwrap() as u32;
            let h = v.height.unwrap() as u32;
            let mut scaled_w = (w as u64 * args.height as u64 / h as u64) as u32;
            // Round up to even (required by most codecs)
            scaled_w += scaled_w % 2;
            let performer_name = v
                .people
                .iter()
                .find(|p| p.person_type == "performer")
                .map(|p| p.name.clone());
            ClipInfo {
                path: p.clone(),
                scaled_width: scaled_w,
                performer_name,
                is_image: v.is_image(),
            }
        })
        .collect();

    // 4. Create scrolling video
    ffmpeg::create_scrolling_video(
        &clips,
        &args.output,
        args.width,
        args.height,
        args.duration,
        &args.codec,
        args.crf,
        args.people.is_empty(),
    )
    .await?;

    Ok(())
}
