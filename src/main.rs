mod cli;
mod config;
mod interactive;
mod ffmpeg;
mod run_params;
mod song;
mod source;
mod types;

use clap::Parser;
use color_eyre::Result;
use tracing::info;

use std::{cmp::Ordering, sync::atomic::AtomicBool};

use crate::{
    cli::Args,
    ffmpeg::VideoParams,
    source::{FetchVideosParams, MediaSource, alexandria::AlexandriaMediaSource},
    types::{ClipInfo, EncodingArgs, generate_output_name},
};

static PROGRESS_HIDDEN: AtomicBool = AtomicBool::new(false);

pub fn progress_hidden() -> bool {
    PROGRESS_HIDDEN.load(std::sync::atomic::Ordering::SeqCst)
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    let seed = args.seed.unwrap_or_else(rand::random);
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
    let source = AlexandriaMediaSource::default();
    let videos = source
        .fetch(FetchVideosParams {
            api_url: &args.api_url,
            content_url: &args.content_url,
            max_clip_duration: args.max_clip_duration,
            desired_count: args.clip_count,
            seed,
            orientation: args.orientation,
            tags: &args.tags,
            people: &args.people,
            with_images: args.with_images,
        })
        .await?;
    info!("Selected {} clips or images", videos.len());
    let paths: Vec<_> = videos
        .iter()
        .map(|v| v.content_url(&args.content_url))
        .collect();

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
    ffmpeg::create_scrolling_video(VideoParams {
        clips: &clips,
        output: &output,
        viewport_height: args.height,
        viewport_width: args.width,
        duration_secs: duration.as_secs() as u32,
        encoding,
        text: args.text,
        audio_path: audio_path.as_deref(),
        easing: args.easing,
    })
    .await?;

    println!("Compilation generated to {output}");

    Ok(())
}
