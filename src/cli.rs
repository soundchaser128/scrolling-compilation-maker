use std::time::Duration;

use clap::Parser;
use tracing::level_filters::LevelFilter;

use crate::{
    ffmpeg::Text,
    types::{AspectRatio, Codec, Effort, Orientation, Quality, ScrollEasing, parse_duration},
};

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
