use std::time::Duration;

use color_eyre::Result;
use inquire::{Confirm, CustomType, Select, Text};

use crate::{
    config::Config,
    run_params::RunParams,
    types::{AspectRatio, Codec, Effort, Orientation, Quality, ScrollEasing, parse_duration},
};

fn parse_comma_list(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn prompt(config: Config) -> Result<RunParams> {
    let tags_input = Text::new("Tags (comma-separated, or empty):")
        .with_default("")
        .prompt()?;
    let tags = parse_comma_list(&tags_input);

    let people_input = Text::new("People/performers (comma-separated, or empty):")
        .with_default("")
        .prompt()?;
    let people = parse_comma_list(&people_input);

    let clip_count: usize = CustomType::new("Number of clips:")
        .with_default(20)
        .prompt()?;

    let duration_str = Text::new("Duration (e.g. 60, 1m, 1m30s):")
        .with_default("1m")
        .with_validator(|input: &str| {
            match parse_duration(input) {
                Ok(_) => Ok(inquire::validator::Validation::Valid),
                Err(e) => Ok(inquire::validator::Validation::Invalid(e.into())),
            }
        })
        .prompt()?;
    let duration = parse_duration(&duration_str).unwrap();

    let orientation = Select::new(
        "Orientation:",
        vec![
            Orientation::Portrait,
            Orientation::Landscape,
            Orientation::Square,
            Orientation::Any,
        ],
    )
    .with_starting_cursor(0)
    .prompt()?;

    let codec = Select::new(
        "Codec:",
        vec![Codec::X264, Codec::Hevc, Codec::Av1],
    )
    .with_starting_cursor(0)
    .prompt()?;

    let quality = Select::new(
        "Quality:",
        vec![
            Quality::Low,
            Quality::Medium,
            Quality::High,
            Quality::VeryHigh,
        ],
    )
    .with_starting_cursor(1)
    .prompt()?;

    let effort = Select::new(
        "Encoding effort:",
        vec![Effort::Low, Effort::Medium, Effort::High],
    )
    .with_starting_cursor(1)
    .prompt()?;

    let with_images = Confirm::new("Include images?")
        .with_default(false)
        .prompt()?;

    let song_input = Text::new("Song URL (empty to skip):")
        .with_default("")
        .prompt()?;
    let song = if song_input.is_empty() {
        None
    } else {
        Some(song_input)
    };

    let crop_input = Text::new("Crop aspect ratio (e.g. 9:16, empty to skip):")
        .with_default("")
        .with_validator(|input: &str| {
            if input.is_empty() {
                return Ok(inquire::validator::Validation::Valid);
            }
            match AspectRatio::parse(input) {
                Ok(_) => Ok(inquire::validator::Validation::Valid),
                Err(e) => Ok(inquire::validator::Validation::Invalid(e.into())),
            }
        })
        .prompt()?;
    let crop = if crop_input.is_empty() {
        None
    } else {
        Some(AspectRatio::parse(&crop_input).unwrap())
    };

    let easing = Select::new(
        "Scroll easing:",
        vec![ScrollEasing::Linear, ScrollEasing::Ease],
    )
    .with_starting_cursor(0)
    .prompt()?;

    let output_input = Text::new("Output file (empty to auto-generate):")
        .with_default("")
        .prompt()?;
    let output = if output_input.is_empty() {
        None
    } else {
        Some(output_input)
    };

    Ok(RunParams {
        clip_count,
        output,
        width: 1920,
        height: 1080,
        duration,
        text: None,
        max_clip_duration: Duration::from_secs(30),
        seed: None,
        orientation,
        tags,
        people,
        with_images,
        crop,
        api_url: config.api_url,
        content_url: config.content_url,
        gpu: config.gpu,
        codec,
        quality,
        effort,
        song,
        easing,
    })
}
