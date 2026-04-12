# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

A Rust CLI tool that creates horizontally-scrolling video compilations from media files hosted on an Alexandria API. It fetches video/image metadata from the API, streams content from a CDN, then uses ffmpeg to stitch clips side-by-side and render a scrolling viewport across them.

## Build & Run

```bash
cargo build                # debug build
cargo build --release      # release build
cargo run -- [OPTIONS]     # run with arguments
```

No tests exist. No linter configuration beyond default `cargo check` / `cargo clippy`.

## External Tool Dependencies

The binary shells out to these tools at runtime (not build time):
- **ffmpeg** / **ffprobe** — video encoding and audio duration probing
- **yt-dlp** — optional, only when `--song <url>` is provided to download audio

## Architecture

The pipeline in `main.rs` is sequential:

1. **Song download** (`song.rs`) — if `--song` is given, downloads audio via yt-dlp and probes its duration with ffprobe; the song duration overrides `--duration`
2. **Fetch metadata** (`source/` trait + `source/alexandria.rs`) — pages through the Alexandria `/api/file` endpoint, filtering by orientation/tags/people, checking each file is reachable via HEAD request on the CDN before accepting it
3. **Compute clip geometry** (`main.rs`) — scales each clip to viewport height, optionally crops to an aspect ratio
4. **Render** (`ffmpeg.rs`) — builds a complex ffmpeg filter graph: scale + optional crop + optional drawtext per input, hstack all inputs into a wide canvas, then a scrolling crop expression over time. Supports software (libx264/libx265/libsvtav1), NVENC, and VideoToolbox encoders.

### Key types

- `MediaSource` trait (`source.rs`) — abstraction for fetching media; `AlexandriaMediaSource` is the only implementation; `stash.rs` is a placeholder
- `EncodingArgs` (`types.rs`) — maps CLI codec/quality/effort/gpu flags to ffmpeg encoder arguments for three backends (software, NVENC, VideoToolbox)
- `ClipInfo` (`types.rs`) — per-clip geometry and metadata used during filter graph construction
- `VideoParams` / `build_filter_graph` (`ffmpeg.rs`) — the core ffmpeg filter graph builder

### CLI

Defined via clap derive in `cli.rs`. Key flags: `--tag`, `--person` (repeatable filters), `--codec`, `--quality`, `--effort`, `--gpu`, `--song`, `--easing`, `--crop`.

## Rust Edition

Uses Rust **2024** edition (`edition = "2024"` in Cargo.toml). This requires a recent nightly or stable toolchain (1.85+).
