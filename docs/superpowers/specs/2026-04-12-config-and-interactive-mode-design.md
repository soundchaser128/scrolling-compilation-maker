# Config File + Interactive Mode

## Summary

Move infrastructure settings (API URL, content URL, GPU, logging) from CLI-only to a `config.toml` file next to the binary. When invoked with no arguments, launch an interactive prompt UI (using `inquire`) instead of the CLI.

## Config File

### Location

Next to the binary, discovered via `std::env::current_exe()`. File is optional — missing file means all defaults apply.

### Format

TOML. New dependency: `toml` crate.

### Schema

All fields optional:

```toml
api_url = "https://alexandria.soundchaser128.com"     # default
content_url = "https://content.r2.soundchaser128.com" # default
gpu = false                                            # default
# log = "info"                                         # omit to disable
```

### Struct

New module `src/config.rs`:

```rust
#[derive(Deserialize, Default)]
pub struct Config {
    pub api_url: Option<String>,
    pub content_url: Option<String>,
    pub gpu: Option<bool>,
    pub log: Option<LevelFilter>,
}
```

`Config::load() -> Result<Config>` reads the file if present, returns `Config::default()` if missing.

## Mode Detection

`std::env::args().len() == 1` (binary name only) triggers interactive mode. Any argument at all uses CLI/clap mode.

## Precedence

CLI args > config file > hardcoded defaults.

For the CLI path: config values act as defaults; any explicitly passed CLI flag overrides them.

## Unified RunParams

Both CLI and interactive paths produce a single `RunParams` struct that `main` consumes:

```rust
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
```

Conversion methods:
- `RunParams::from_cli(args: Args, config: Config) -> RunParams` — merges CLI args with config defaults
- `interactive::prompt(config: Config) -> Result<RunParams>` — interactive prompts, config provides defaults for infra fields

## Interactive Mode

New module `src/interactive.rs`. Uses the `inquire` crate (already a dependency).

Prompt sequence:

1. **Tags** — `Text`, comma-separated, can be empty
2. **People** — `Text`, comma-separated, can be empty
3. **Clip count** — `CustomType<usize>`, default 20
4. **Duration** — `Text`, default "1m", validated with `parse_duration`
5. **Orientation** — `Select` from enum variants (portrait/landscape/square/any)
6. **Codec** — `Select` (x264/hevc/av1)
7. **Quality** — `Select` (low/medium/high/very-high)
8. **Effort** — `Select` (low/medium/high)
9. **Include images** — `Confirm`, default no
10. **Song URL** — `Text`, optional (empty = skip)
11. **Crop** — `Text`, optional (empty = skip), validated with `AspectRatio::parse`
12. **Easing** — `Select` (linear/ease)
13. **Output file** — `Text`, optional (empty = auto-generate)

Width/height, max_clip_duration, seed, text, and download_concurrency use hardcoded defaults (same as current clap defaults). These are power-user options unlikely to be needed interactively.

## Changes to main.rs

```
fn main():
    config = Config::load()?

    if std::env::args().len() == 1:
        setup logging from config.log
        params = interactive::prompt(config)?
    else:
        args = Args::parse()
        params = RunParams::from_cli(args, config)
        setup logging from params

    // rest of pipeline unchanged, uses params.field instead of args.field
```

## New Files

- `src/config.rs` — Config struct + load()
- `src/interactive.rs` — prompt function
- `src/run_params.rs` — RunParams struct + from_cli()

## Modified Files

- `Cargo.toml` — add `toml` dependency
- `src/main.rs` — mode detection, use RunParams instead of Args
- `src/cli.rs` — remove config-file fields from clap defaults where config takes over (api_url, content_url, gpu, log remain as CLI overrides but their defaults come from config)

## New Dependencies

- `toml` (TOML deserialization)
