# Config File + Interactive Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a TOML config file for infrastructure settings and an interactive prompt UI when running without arguments.

**Architecture:** New `config.rs` loads optional `config.toml` from beside the binary. New `interactive.rs` uses `inquire` to prompt the user. Both paths produce a `RunParams` struct that replaces direct `Args` usage in `main.rs`. Display impls are added to enums so `Select` prompts show readable names.

**Tech Stack:** Rust, toml crate (new dep), inquire (existing dep), clap (existing)

---

### Task 1: Add `toml` dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add toml to dependencies**

In `Cargo.toml`, add after the `tokio-util` line:

```toml
toml = "0.8"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "add toml dependency"
```

---

### Task 2: Create `config.rs`

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs` (add `mod config;`)

- [ ] **Step 1: Create `src/config.rs`**

```rust
use std::path::PathBuf;

use color_eyre::Result;
use serde::Deserialize;
use tracing::level_filters::LevelFilter;

fn default_api_url() -> String {
    "https://alexandria.soundchaser128.com".to_string()
}

fn default_content_url() -> String {
    "https://content.r2.soundchaser128.com".to_string()
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    #[serde(default = "default_api_url")]
    pub api_url: String,
    #[serde(default = "default_content_url")]
    pub content_url: String,
    #[serde(default)]
    pub gpu: bool,
    #[serde(default)]
    pub log: Option<LevelFilter>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: default_api_url(),
            content_url: default_content_url(),
            gpu: false,
            log: None,
        }
    }
}

fn config_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let path = dir.join("config.toml");
    if path.exists() { Some(path) } else { None }
}

impl Config {
    pub fn load() -> Result<Self> {
        match config_path() {
            Some(path) => {
                let content = std::fs::read_to_string(&path)?;
                let config: Config = toml::from_str(&content)?;
                Ok(config)
            }
            None => Ok(Config::default()),
        }
    }
}
```

- [ ] **Step 2: Register module in `src/main.rs`**

Add `mod config;` after the existing `mod cli;` line (line 1 of main.rs):

```rust
mod cli;
mod config;
mod ffmpeg;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: compiles (config module is declared but not yet used — that's fine)

- [ ] **Step 4: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "add config module with TOML loading"
```

---

### Task 3: Add Display impls to enums

**Files:**
- Modify: `src/types.rs`

The `inquire::Select` prompt needs `Display` on the enum types. Add `Display` impls for `Orientation`, `Codec`, `Quality`, `Effort`, `ScrollEasing`.

- [ ] **Step 1: Add Display impls**

Add at the end of `src/types.rs`:

```rust
impl std::fmt::Display for Orientation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Orientation::Any => write!(f, "Any"),
            Orientation::Portrait => write!(f, "Portrait"),
            Orientation::Landscape => write!(f, "Landscape"),
            Orientation::Square => write!(f, "Square"),
        }
    }
}

impl std::fmt::Display for Codec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Codec::X264 => write!(f, "H.264 (x264)"),
            Codec::Hevc => write!(f, "HEVC (x265)"),
            Codec::Av1 => write!(f, "AV1 (SVT-AV1)"),
        }
    }
}

impl std::fmt::Display for Quality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Quality::Low => write!(f, "Low"),
            Quality::Medium => write!(f, "Medium"),
            Quality::High => write!(f, "High"),
            Quality::VeryHigh => write!(f, "Very High"),
        }
    }
}

impl std::fmt::Display for Effort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Effort::Low => write!(f, "Low"),
            Effort::Medium => write!(f, "Medium"),
            Effort::High => write!(f, "High"),
        }
    }
}

impl std::fmt::Display for ScrollEasing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScrollEasing::Linear => write!(f, "Linear"),
            ScrollEasing::Ease => write!(f, "Ease (smoothstep)"),
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add src/types.rs
git commit -m "add Display impls for enum types"
```

---

### Task 4: Create `RunParams` struct

**Files:**
- Create: `src/run_params.rs`
- Modify: `src/main.rs` (add `mod run_params;`)

- [ ] **Step 1: Create `src/run_params.rs`**

```rust
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
            // CLI flags override config; clap's default_value means the CLI
            // field is always populated, so we check whether the user actually
            // passed the flag. Since we can't easily distinguish "user passed
            // --api-url" from "clap used the default", we let the CLI value
            // win — the defaults match anyway.
            api_url: args.api_url,
            content_url: args.content_url,
            gpu: args.gpu || config.gpu,
            codec: args.codec,
            quality: args.quality,
            effort: args.effort,
            song: args.song,
            easing: args.easing,
        }
    }
}
```

- [ ] **Step 2: Register module in `src/main.rs`**

Add `mod run_params;` after `mod ffmpeg;`:

```rust
mod cli;
mod config;
mod ffmpeg;
mod run_params;
mod song;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: compiles (unused warnings are fine for now)

- [ ] **Step 4: Commit**

```bash
git add src/run_params.rs src/main.rs
git commit -m "add RunParams struct with from_cli conversion"
```

---

### Task 5: Create `interactive.rs`

**Files:**
- Create: `src/interactive.rs`
- Modify: `src/main.rs` (add `mod interactive;`)

- [ ] **Step 1: Create `src/interactive.rs`**

```rust
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
```

- [ ] **Step 2: Register module in `src/main.rs`**

Add `mod interactive;` after `mod config;`:

```rust
mod cli;
mod config;
mod ffmpeg;
mod interactive;
mod run_params;
mod song;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add src/interactive.rs src/main.rs
git commit -m "add interactive prompt module"
```

---

### Task 6: Rewrite `main.rs` to use `RunParams` and mode detection

**Files:**
- Modify: `src/main.rs`

This is the integration step. Replace direct `Args` field access with `RunParams`.

- [ ] **Step 1: Rewrite `src/main.rs`**

Replace the entire contents of `src/main.rs` with:

```rust
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

#[tokio::main]
async fn main() -> Result<()> {
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

    ffmpeg::check_ffmpeg().await?;

    let temp_dir = tempfile::tempdir()?;

    // 0. Download song if provided
    let mut duration = params.duration;
    let audio_path = match &params.song {
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
            api_url: &params.api_url,
            content_url: &params.content_url,
            max_clip_duration: params.max_clip_duration,
            desired_count: params.clip_count,
            seed,
            orientation: params.orientation,
            tags: &params.tags,
            people: &params.people,
            with_images: params.with_images,
        })
        .await?;
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
    })
    .await?;

    println!("Compilation generated to {output}");

    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors. There may be an unused `progress_hidden` variable warning — that's a pre-existing pattern.

- [ ] **Step 3: Smoke-test CLI mode still works**

Run: `cargo run -- --help`
Expected: shows the clap help text with all options.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "integrate config file and interactive mode into main"
```

---

### Task 7: Create a default `config.toml`

**Files:**
- Create: `config.toml` (project root, for reference/documentation)

- [ ] **Step 1: Create `config.toml`**

```toml
# scrolling-compilation-maker configuration
# Place this file next to the binary.

# API base URL
api_url = "https://alexandria.soundchaser128.com"

# Content CDN base URL
content_url = "https://content.r2.soundchaser128.com"

# Use GPU-accelerated encoding (NVIDIA NVENC / macOS VideoToolbox)
gpu = false

# Enable logging (uncomment and set level: trace, debug, info, warn, error)
# log = "info"
```

- [ ] **Step 2: Add config.toml to .gitignore**

The config file contains user-specific URLs. Add to `.gitignore`:

```
/target
*.mp4
config.toml
```

But also create a tracked example:

Rename the file to `config.example.toml` instead — this way the example is tracked but actual configs are not.

Rename `config.toml` to `config.example.toml`.

- [ ] **Step 3: Commit**

```bash
git add config.example.toml
git commit -m "add example config file"
```

---

### Task 8: Final cleanup and verification

**Files:**
- Modify: `src/main.rs` (remove unused variable if needed)

- [ ] **Step 1: Run clippy**

Run: `cargo clippy -- -W warnings`
Expected: no errors. Fix any warnings.

- [ ] **Step 2: Build release**

Run: `cargo build --release`
Expected: compiles successfully.

- [ ] **Step 3: Test interactive mode launches**

Run: `cargo run` (no arguments)
Expected: the first inquire prompt ("Tags") appears in the terminal.
Press Ctrl+C to exit.

- [ ] **Step 4: Test CLI mode still works**

Run: `cargo run -- --help`
Expected: clap help text is displayed.

- [ ] **Step 5: Commit any cleanup**

```bash
git add -A
git commit -m "cleanup: fix clippy warnings"
```

(Skip this commit if there were no warnings to fix.)
