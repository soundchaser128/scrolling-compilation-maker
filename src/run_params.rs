use std::time::Duration;

use crate::{
    cli::Args,
    config::Config,
    ffmpeg::Text,
    types::{AspectRatio, Codec, Effort, Orientation, Quality, ScrollEasing},
};

pub struct RunParams {
    pub clip_count: usize,
    pub output: Option<String>,
    pub width: u32,
    pub height: u32,
    pub duration: Duration,
    pub text: Option<Text>,
    pub max_clip_duration: Duration,
    pub seed: Option<f64>,
    pub orientation: Orientation,
    pub tags: Vec<String>,
    pub people: Vec<String>,
    pub with_images: bool,
    pub crop: Option<AspectRatio>,
    pub api_url: String,
    pub content_url: String,
    pub gpu: bool,
    pub codec: Codec,
    pub quality: Quality,
    pub effort: Effort,
    pub song: Option<String>,
    pub easing: ScrollEasing,
}

impl RunParams {
    pub fn from_cli(args: Args, config: Config) -> Self {
        Self {
            clip_count: args.clip_count,
            output: args.output,
            width: args.width,
            height: args.height,
            duration: args.duration,
            text: args.text,
            max_clip_duration: args.max_clip_duration,
            seed: args.seed,
            orientation: args.orientation,
            tags: args.tags,
            people: args.people,
            with_images: args.with_images,
            crop: args.crop,
            api_url: config.api_url,
            content_url: config.content_url,
            gpu: args.gpu || config.gpu,
            codec: args.codec,
            quality: args.quality,
            effort: args.effort,
            song: args.song,
            easing: args.easing,
        }
    }
}
