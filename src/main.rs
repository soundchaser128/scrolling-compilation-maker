mod cli;
mod config;
mod ffmpeg;
mod interactive;
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
    config::Config,
    ffmpeg::VideoParams,
    run_params::RunParams,
    source::{FetchVideosParams, MediaSource, alexandria::AlexandriaMediaSource},
    types::{ClipInfo, EncodingArgs, generate_output_name},
};

static PROGRESS_HIDDEN: AtomicBool = AtomicBool::new(false);

pub fn progress_hidden() -> bool {
    PROGRESS_HIDDEN.load(std::sync::atomic::Ordering::SeqCst)
}

fn setup_logging(log: Option<tracing::level_filters::LevelFilter>) -> Result<()> {
    let has_logging = log.is_some();
    if let Some(filter) = log {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive(format!("scrolling_compilation_maker={filter}").parse()?),
            )
            .init();
    }
    PROGRESS_HIDDEN.store(has_logging, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let config = Config::load()?;

    let params = if std::env::args().len() == 1 {
        // Interactive mode: no CLI arguments
        setup_logging(config.log)?;
        interactive::prompt(config)?
    } else {
        // CLI mode
        let args = Args::parse();
        let log = args.log;
        let params = RunParams::from_cli(args, config);
        setup_logging(log)?;
        params
    };

    let seed = params.seed.unwrap_or_else(rand::random);
    let output = params
        .output
        .unwrap_or_else(|| generate_output_name(&params.tags, &params.people));

    info!("Using seed: {seed}");
    info!("Output file: {output}");

    ffmpeg::check_ffmpeg()?;

    let temp_dir = tempfile::tempdir()?;

    // 0. Download song if provided
    let mut duration = params.duration;
    let audio_path = match &params.song {
        Some(url) => {
            let path = song::download_song(url, temp_dir.path())?;
            let song_duration = song::probe_duration(&path)?;
            info!("Song duration: {:.1}s", song_duration.as_secs_f64());
            duration = song_duration;
            Some(path)
        }
        None => None,
    };

    // 1. Fetch video metadata
    info!("Fetching video metadata...");
    let source = AlexandriaMediaSource::new(params.api_url.clone(), params.content_url.clone());
    let videos = source.fetch(FetchVideosParams {
        max_clip_duration: params.max_clip_duration,
        desired_count: params.clip_count,
        seed,
        orientation: params.orientation,
        tags: &params.tags,
        people: &params.people,
        with_images: params.with_images,
    })?;
    info!("Selected {} clips or images", videos.len());
    let paths: Vec<_> = videos
        .iter()
        .map(|v| v.content_url(&params.content_url))
        .collect();

    // 3. Compute clip info with scaled dimensions
    let crop_width = params.crop.as_ref().map(|a| a.crop_width(params.height));
    let mut clips: Vec<ClipInfo> = videos
        .into_iter()
        .zip(paths.iter())
        .map(|(v, p)| {
            let w = v.width.unwrap() as u32;
            let h = v.height.unwrap() as u32;
            let mut scaled_w = (w as u64 * params.height as u64 / h as u64) as u32;
            scaled_w += scaled_w % 2;
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
    let encoding = EncodingArgs::new(&params.codec, &params.quality, &params.effort, params.gpu);
    ffmpeg::create_scrolling_video(VideoParams {
        clips: &clips,
        output: &output,
        viewport_height: params.height,
        viewport_width: params.width,
        duration_secs: duration.as_secs() as u32,
        encoding,
        text: params.text,
        audio_path: audio_path.as_deref(),
        easing: params.easing,
    })?;

    println!("Compilation generated to {output}");

    Ok(())
}
