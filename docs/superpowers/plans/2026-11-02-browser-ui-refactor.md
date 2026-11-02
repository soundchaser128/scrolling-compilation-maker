# Browser UI + Semi-Automated Clip Selection Refactor

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the CLI/TUI `scrolling-compilation-maker` into a browser-based tool that produces shareable 60-second scrolling compilations from an Alexandria media pool, with the user picking which clips (and which time ranges within them) to use.

**Architecture:** A Rust `axum`/`tokio` backend replaces the CLI/TUI entrypoints but reuses the ffmpeg filter-graph builder and Alexandria client logic. A SvelteKit frontend drives a 3-step wizard (filters → clip pool + time-range selection → render + download). The backend exposes a small JSON API (`/api/pool`, `/api/render`, `/api/render/{id}`, …) and runs ffmpeg as background jobs with progress polling. The scrolling viewport render model is unchanged; the new "time range" concept is implemented by trimming each selected clip to a local segment that then loops inside the existing filter graph.

**Tech Stack:** Rust (axum, tokio, reqwest async, futures), SvelteKit (Svelte 5 runes), ffmpeg/ffprobe (external), yt-dlp (optional, unchanged).

---

## Key Assumptions & Decisions (confirm before large build-out)

1. **Render model stays "scrolling viewport".** The output remains a fixed-duration (default 60s) horizontal pan across a hstacked strip of clips. Per-clip time ranges define *which segment of each clip loops* while the viewport passes over it — they do NOT change the overall 60s pacing.
2. **Monorepo layout.** New `server/` crate lives beside the existing root crate during development; the old `src/` CLI/TUI is deleted in the final milestone. `web/` holds the SvelteKit app. Output videos go to `server/output/`.
3. **Duration fixed at 60s** (configurable via env + a render-form field), matching RedGifs.
4. **Stateless metadata.** The frontend sends full clip metadata (URL, dims, mime, performers/tags) back in the render request, so the backend never re-fetches by id.
5. **Trimming uses `-c copy`** (stream copy) for speed. Cuts land on keyframes (imprecision up to a GOP, typically a few seconds). A re-encode fallback is noted as a future option — not in scope for this plan.
6. **In-memory only.** Clip pool + selection + jobs live in process memory. Page reload loses selection (acceptable for a local single-user tool).
7. **Preview.** The browser plays clips directly from the CDN `content_url` (R2 supports HTTP range requests, which plain `<video>` playback needs). Backend generates cached poster JPEGs for the grid.

---

## Repository Layout (target end state)

```
scrolling-compilation-maker/
├── server/
│   ├── Cargo.toml
│   ├── output/                 # generated .mp4 files (gitignored)
│   └── src/
│       ├── main.rs             # axum bootstrap + router
│       ├── config.rs           # env-based config
│       ├── state.rs            # AppState (source client, jobs, dirs)
│       ├── error.rs            # ApiError -> IntoResponse
│       ├── types.rs            # ported MediaFile/ClipInfo/EncodingArgs/enums
│       ├── source.rs           # async MediaSource trait
│       ├── source/
│       │   └── alexandria.rs   # async Alexandria client
│       ├── ffmpeg.rs           # build_filter_graph (ported), trim_clip, render
│       ├── jobs.rs             # job registry + status
│       └── api.rs              # routes + handlers + DTOs
├── web/
│   ├── package.json
│   ├── vite.config.ts          # /api proxy -> 127.0.0.1:3001
│   ├── svelte.config.js
│   ├── tsconfig.json
│   └── src/
│       ├── app.html
│       ├── app.d.ts
│       ├── lib/
│       │   ├── types.ts        # mirror of backend DTOs
│       │   ├── api.ts          # typed fetch helpers
│       │   └── components/
│       │       ├── FilterForm.svelte
│       │       ├── ClipGrid.svelte
│       │       ├── ClipCard.svelte
│       │       ├── RenderForm.svelte
│       │       └── RenderProgress.svelte
│       └── routes/
│           ├── +layout.svelte
│           └── +page.svelte    # 3-step wizard
└── (old root src/ Cargo.toml deleted in Milestone 6)
```

---

# Milestone 0 — Project Scaffold

### Task 0.1: Create the `server` crate

**Files:**
- Create: `server/Cargo.toml`
- Create: `server/src/main.rs`

- [ ] **Step 1: Write `server/Cargo.toml`**

```toml
[package]
name = "scm-server"
version = "0.1.0"
edition = "2024"

[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.6", features = ["cors", "trace"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
color-eyre = "0.6"
uuid = { version = "1", features = ["serde", "v4"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror = "2"
rand = "0.9"
futures = "0.3"
tokio-util = { version = "0.7", features = ["io"] }
```

- [ ] **Step 2: Write `server/src/main.rs` (hello server, replaced later)**

```rust
use axum::{Router, routing::get};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let app = Router::new().route("/", get(|| async { "scm-server" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 3: Verify**

Run: `cargo build --manifest-path server/Cargo.toml`
Expected: compiles, no errors.

- [ ] **Step 4: Commit**

```bash
git add server/Cargo.toml server/src/main.rs server/Cargo.lock
git commit -m "scaffold axum server crate"
```

### Task 0.2: Create the SvelteKit app

- [ ] **Step 1: Scaffold with npm**

Run: `npm create svelte@latest web` (choose: TypeScript, ESLint off, Prettier off, no add-ons, "Skeleton" template)

- [ ] **Step 2: Install**

Run: `cd web && npm install`

- [ ] **Step 3: Verify**

Run: `cd web && npm run dev` (kill after confirming it serves)
Expected: dev server starts on http://localhost:5173

- [ ] **Step 4: Commit**

```bash
git add web/
git commit -m "scaffold sveltekit frontend"
```

### Task 0.3: Ignore build artifacts

**Files:**
- Modify: `.gitignore`

- [ ] **Step 1: Append**

```gitignore
/server/target
/server/output
/web/node_modules
/web/build
/web/.svelte-kit
```

- [ ] **Step 2: Commit**

```bash
git add .gitignore
git commit -m "ignore server + web build artifacts"
```

---

# Milestone 1 — Shared Types, Config, Errors (server)

### Task 1.1: Port `types.rs`

**Files:**
- Create: `server/src/types.rs`
- Modify: `server/src/main.rs` (add `mod types;`)

- [ ] **Step 1: Copy `src/types.rs` → `server/src/types.rs`** and apply these exact changes:

1. **Remove** the `clap::ValueEnum` derives from `Orientation`, `ScrollEasing`, `Codec`, `Quality`, `Effort`, and the `Text` enum (in `ffmpeg.rs`, handled in Task 3.1).
2. **Remove** `use clap::ValueEnum;`.
3. **Add** `Serialize` + `Deserialize` to the request enums with lowercase serde names:

```rust
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Orientation { Any, Portrait, Landscape, Square }

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScrollEasing { Linear, Ease }

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Codec { X264, Hevc, Av1 }

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality { Low, Medium, High, VeryHigh }

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effort { Low, Medium, High }
```

4. **Remove** `generate_output_name`, `random_id`, `IdFormat`, `ADJECTIVES`, `NOUNS`, `slugify`, and the `use rand::prelude::IndexedRandom;` + `use uuid::Uuid;` imports if now unused (output naming moves to a UUID in Task 3.4/4.1). `uuid::Uuid` stays if `MediaFile.id` still needs it (it does).
5. **Keep** `AspectRatio` (with `parse` + `crop_width`), `parse_duration`, `PageResponse`, `PageInfo`, `FileType`, `MediaFile` + `content_url`, `Person`, `ClipInfo`, `EncodingArgs` + its `Display` impls exactly as-is.

- [ ] **Step 2: Verify**

Run: `cargo check --manifest-path server/Cargo.toml`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add server/src/types.rs server/src/main.rs
git commit -m "port types.rs to server crate"
```

### Task 1.2: Create `config.rs`

**Files:**
- Create: `server/src/config.rs`
- Modify: `server/src/main.rs` (add `mod config;`)

- [ ] **Step 1: Write `server/src/config.rs`**

```rust
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub api_url: String,
    pub content_url: String,
    pub bind_addr: String,
    pub output_dir: PathBuf,
    pub gpu: bool,
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            api_url: env_or("ALEXANDRIA_API_URL", "https://alexandria.soundchaser128.com"),
            content_url: env_or("CONTENT_URL", "https://content.r2.soundchaser128.com"),
            bind_addr: env_or("BIND_ADDR", "127.0.0.1:3001"),
            output_dir: PathBuf::from(env_or("OUTPUT_DIR", "./output")),
            gpu: env_or("GPU", "false") == "true",
        }
    }
}
```

- [ ] **Step 2: Verify + commit**

Run: `cargo check --manifest-path server/Cargo.toml` then:
```bash
git add server/src/config.rs server/src/main.rs
git commit -m "add env-based config"
```

### Task 1.3: Create `error.rs`

**Files:**
- Create: `server/src/error.rs`
- Modify: `server/src/main.rs` (add `mod error;`)

- [ ] **Step 1: Write `server/src/error.rs`**

```rust
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub type ApiResult<T> = std::result::Result<T, ApiError>;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Internal(String),
}

impl From<color_eyre::Report> for ApiError {
    fn from(e: color_eyre::Report) -> Self {
        ApiError::Internal(format!("{e:#}"))
    }
}
impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self {
        ApiError::Internal(e.to_string())
    }
}
impl From<std::io::Error> for ApiError {
    fn from(e: std::io::Error) -> Self {
        ApiError::Internal(e.to_string())
    }
}
impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::BadRequest(e.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let msg = self.to_string();
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}
```

- [ ] **Step 2: Verify + commit**

```bash
cargo check --manifest-path server/Cargo.toml
git add server/src/error.rs server/src/main.rs
git commit -m "add ApiError type"
```

---

# Milestone 2 — Async Alexandria Source + Pool Endpoint

### Task 2.1: Create `source.rs` (async trait)

**Files:**
- Create: `server/src/source.rs`
- Create: `server/src/source/mod.rs` → not needed; keep `source.rs` with `pub mod alexandria;`
- Modify: `server/src/main.rs` (add `mod source;`)

- [ ] **Step 1: Write `server/src/source.rs`**

```rust
use color_eyre::Result;

use crate::types::{MediaFile, Orientation};

pub mod alexandria;

pub struct PoolParams {
    pub pool_size: usize,
    pub max_clip_duration_ms: u64,
    pub seed: f64,
    pub orientation: Orientation,
    pub tags: Vec<String>,
    pub people: Vec<String>,
    pub with_images: bool,
}

#[allow(async_fn_in_trait)]
pub trait MediaSource: Send + Sync {
    async fn fetch_pool(&self, params: PoolParams) -> Result<Vec<MediaFile>>;
    async fn fetch_people(&self, prefix: Option<&str>) -> Result<Vec<String>>;
    async fn fetch_tags(&self, prefix: Option<&str>) -> Result<Vec<String>>;
}
```

- [ ] **Step 2: Verify + commit**

```bash
cargo check --manifest-path server/Cargo.toml
git add server/src/source.rs server/src/main.rs
git commit -m "add async MediaSource trait"
```

### Task 2.2: Create `source/alexandria.rs` (async client)

**Files:**
- Create: `server/src/source/alexandria.rs`

- [ ] **Step 1: Write `server/src/source/alexandria.rs`** (async conversion of the existing `src/source/alexandria.rs`, with a `pool_size` gathering pass + concurrent reachability filter):

```rust
use std::sync::Arc;

use color_eyre::{Result, eyre::bail};
use futures::stream::{self, StreamExt};
use reqwest::{Client, Url};
use serde::Deserialize;
use tracing::warn;

use crate::{
    source::{MediaSource, PoolParams},
    types::{MediaFile, PageResponse},
};

#[derive(Clone)]
pub struct AlexandriaMediaSource {
    api_url: Arc<str>,
    content_url: Arc<str>,
    client: Client,
}

impl AlexandriaMediaSource {
    pub fn new(api_url: String, content_url: String) -> Self {
        Self {
            api_url: api_url.into(),
            content_url: content_url.into(),
            client: Client::new(),
        }
    }

    async fn reachable(&self, file: &MediaFile) -> bool {
        let Ok(resp) = self.client.head(file.content_url(&self.content_url)).send().await else {
            return false;
        };
        resp.status().is_success()
    }
}

#[allow(async_fn_in_trait)]
impl MediaSource for AlexandriaMediaSource {
    async fn fetch_pool(&self, params: PoolParams) -> Result<Vec<MediaFile>> {
        let mut candidates: Vec<MediaFile> = Vec::new();
        let mut page = 0u32;
        let seed = params.seed.to_string();

        loop {
            let mut url = Url::parse(&self.api_url)?;
            url.set_path("/api/file");
            {
                let mut q = url.query_pairs_mut();
                q.append_pair("sort", "random");
                q.append_pair("fileType", "video");
                q.append_pair("size", "50");
                q.append_pair("page", &page.to_string());
                q.append_pair("seed", &seed);
                q.append_pair("withTags", "true");
                if params.with_images {
                    q.append_pair("fileType", "image");
                }
                if let Some(o) = params.orientation.as_api_param() {
                    q.append_pair("orientation", o);
                }
                for p in &params.people {
                    q.append_pair("person", p);
                }
                for t in &params.tags {
                    q.append_pair("tag", t);
                }
            }

            let resp = self.client.get(url).send().await?;
            if !resp.status().is_success() {
                bail!(
                    "API request failed with status {}: {}",
                    resp.status(),
                    resp.text().await.unwrap_or_default()
                );
            }
            let page_response: PageResponse<MediaFile> = resp.json().await?;
            let total_pages = page_response.page.total_pages;

            for m in page_response.content {
                let Some(_) = m.width.zip(m.height).zip(m.duration) else {
                    continue; // missing dimensions/duration
                };
                if m.duration.unwrap() > params.max_clip_duration_ms {
                    continue;
                }
                candidates.push(m);
            }

            page += 1;
            // Gather 3x the pool size to survive reachability filtering
            if page >= total_pages || candidates.len() >= params.pool_size * 3 {
                break;
            }
        }

        let src = self.clone();
        let pool: Vec<MediaFile> = stream::iter(candidates.into_iter())
            .map(|m| {
                let src = src.clone();
                async move { if src.reachable(&m).await { Some(m) } else { None } }
            })
            .buffer_unordered(8)
            .filter_map(|m| async move { m })
            .take(params.pool_size)
            .collect()
            .await;

        if pool.is_empty() {
            bail!("No suitable media found");
        }
        warn!("Returned {} of {} requested", pool.len(), params.pool_size);
        Ok(pool)
    }

    async fn fetch_people(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        let mut url = Url::parse(&format!("{}/api/people/autocomplete", self.api_url))?;
        if let Some(prefix) = prefix {
            url.query_pairs_mut().append_pair("query", prefix);
        }
        let people: Vec<PersonResponse> = self.client.get(url).send().await?.json().await?;
        Ok(people.into_iter().map(|p| p.name).collect())
    }

    async fn fetch_tags(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        let mut url = Url::parse(&format!("{}/api/tag", self.api_url))?;
        if let Some(prefix) = prefix {
            url.query_pairs_mut().append_pair("query", prefix);
        }
        let tags: Vec<TagCompletion> = self.client.get(url).send().await?.json().await?;
        Ok(tags.into_iter().map(|t| t.name).collect())
    }
}

#[derive(Deserialize)]
struct PersonResponse {
    name: String,
}

#[derive(Deserialize)]
struct TagCompletion {
    name: String,
    category: Option<String>,
    count: i64,
}
```

- [ ] **Step 2: Verify + commit**

```bash
cargo check --manifest-path server/Cargo.toml
git add server/src/source/alexandria.rs
git commit -m "async alexandria source with concurrent reachability check"
```

### Task 2.3: Pool + autocomplete endpoints

**Files:**
- Create: `server/src/api.rs`
- Create: `server/src/state.rs`
- Modify: `server/src/main.rs`

- [ ] **Step 1: Write `server/src/state.rs`**

```rust
use std::path::PathBuf;
use std::sync::Arc;

use crate::{config::Config, jobs::JobRegistry, source::alexandria::AlexandriaMediaSource};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub source: AlexandriaMediaSource,
    pub jobs: Arc<JobRegistry>,
    pub work_dir: Arc<PathBuf>,
}

impl AppState {
    pub fn new(config: Config) -> color_eyre::Result<Self> {
        let source = AlexandriaMediaSource::new(config.api_url.clone(), config.content_url.clone());
        let work_dir = Arc::new(std::env::temp_dir().join(format!("scm-{}", uuid::Uuid::new_v4())));
        std::fs::create_dir_all(&*work_dir)?;
        std::fs::create_dir_all(&config.output_dir)?;
        Ok(Self {
            config,
            source,
            jobs: Arc::new(JobRegistry::default()),
            work_dir,
        })
    }
}
```

- [ ] **Step 2: Write `server/src/api.rs`** (pool + autocomplete + config only; render endpoints added later):

```rust
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use crate::{
    error::ApiResult,
    state::AppState,
    types::{MediaFile, Orientation},
};

#[derive(Serialize)]
pub struct ClipCandidate {
    pub id: Uuid,
    pub content_url: String,
    pub file_type: String,
    pub width: i32,
    pub height: i32,
    pub duration_ms: u64,
    pub tags: Vec<String>,
    pub performers: Vec<String>,
    pub popularity: f32,
}

fn candidate(m: &MediaFile, content_base: &str) -> ClipCandidate {
    ClipCandidate {
        id: m.id,
        content_url: m.content_url(content_base),
        file_type: match m.file_type {
            crate::types::FileType::Image => "image".into(),
            _ => "video".into(),
        },
        width: m.width.unwrap_or(0),
        height: m.height.unwrap_or(0),
        duration_ms: m.duration.unwrap_or(0),
        tags: m.tags.clone(),
        performers: m.people.iter().map(|p| p.name.clone()).collect(),
        popularity: m.popularity,
    }
}

#[derive(Deserialize)]
pub struct PoolRequest {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub people: Vec<String>,
    #[serde(default)]
    pub orientation: Orientation,
    #[serde(default)]
    pub with_images: bool,
    #[serde(default = "default_max_duration")]
    pub max_clip_duration_ms: u64,
    #[serde(default = "default_pool_size")]
    pub pool_size: usize,
    pub seed: Option<f64>,
}

fn default_max_duration() -> u64 { 30_000 }
fn default_pool_size() -> usize { 50 }

#[derive(Deserialize)]
pub struct AutocompleteQuery {
    pub query: Option<String>,
}

async fn create_pool(
    State(state): State<AppState>,
    Json(req): Json<PoolRequest>,
) -> ApiResult<Json<Vec<ClipCandidate>>> {
    let seed = req.seed.unwrap_or_else(rand::random);
    let files = state
        .source
        .fetch_pool(crate::source::PoolParams {
            pool_size: req.pool_size,
            max_clip_duration_ms: req.max_clip_duration_ms,
            seed,
            orientation: req.orientation,
            tags: req.tags,
            people: req.people,
            with_images: req.with_images,
        })
        .await?;
    let candidates: Vec<ClipCandidate> = files
        .iter()
        .map(|m| candidate(m, &state.config.content_url))
        .collect();
    Ok(Json(candidates))
}

async fn autocomplete_tags(
    State(state): State<AppState>,
    Query(q): Query<AutocompleteQuery>,
) -> ApiResult<Json<Vec<String>>> {
    let tags = state.source.fetch_tags(q.query.as_deref()).await?;
    Ok(Json(tags))
}

async fn autocomplete_people(
    State(state): State<AppState>,
    Query(q): Query<AutocompleteQuery>,
) -> ApiResult<Json<Vec<String>>> {
    let people = state.source.fetch_people(q.query.as_deref()).await?;
    Ok(Json(people))
}

async fn get_config(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "gpu": state.config.gpu,
        "outputDir": state.config.output_dir.to_string_lossy(),
    }))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/config", get(get_config))
        .route("/api/tags", get(autocomplete_tags))
        .route("/api/people", get(autocomplete_people))
        .route("/api/pool", post(create_pool))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
```

- [ ] **Step 3: Wire router in `main.rs`**

Replace the `hello` router in `server/src/main.rs` with:

```rust
mod api;
mod config;
mod error;
mod jobs;
mod source;
mod state;
mod types;

use axum::Router;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = config::Config::from_env();
    let state = state::AppState::new(config)?;
    let app: Router = api::router(state);

    let listener = tokio::net::TcpListener::bind(
        std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3001".into()),
    )
    .await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
```

> Note: `jobs` is referenced by `main.rs` before `jobs.rs` exists. Create a placeholder `server/src/jobs.rs` now with `#[derive(Default)] pub struct JobRegistry;` (replaced in Task 3.4).

- [ ] **Step 4: Verify**

Run: `cargo build --manifest-path server/Cargo.toml` then start it and probe:

```bash
./server/target/debug/scm-server &
curl -s 'http://127.0.0.1:3001/api/tags?query=a' | head -c 300
curl -s -X POST http://127.0.0.1:3001/api/pool -H 'content-type: application/json' -d '{"pool_size":5,"tags":[],"people":[]}' | head -c 300
```

Expected: 200 JSON for tags; pool returns an array of up to 5 `ClipCandidate` objects.

- [ ] **Step 5: Commit**

```bash
git add server/src/
git commit -m "add pool + autocomplete endpoints"
```

---

# Milestone 3 — ffmpeg Pipeline (trim + render + jobs)

### Task 3.1: Port `ffmpeg.rs` (filter graph + encoding)

**Files:**
- Create: `server/src/ffmpeg.rs`
- Modify: `server/src/main.rs` (add `mod ffmpeg;`)

- [ ] **Step 1: Copy `src/ffmpeg.rs` → `server/src/ffmpeg.rs`**, then:

1. Remove `use clap::ValueEnum;` and change the `Text` enum:

```rust
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Text { Performers, Tags }
```

2. Delete the `indicatif` progress-bar code (`ffmpeg_progress_bar`, `ProgressBar` imports, `crate::progress_hidden()` calls, `indicatif`/`regex` imports) — progress moves to async stdout parsing in Task 3.3.
3. Keep `build_filter_graph` and `escape_drawtext` **verbatim**. Keep `check_ffmpeg` (sync version is fine for a startup check).
4. Delete the blocking `create_scrolling_video` (replaced by the async version in Task 3.3).
5. Keep `ClipInfo` in `types.rs` (already ported) — no change needed here.

- [ ] **Step 2: Verify + commit**

```bash
cargo check --manifest-path server/Cargo.toml
git add server/src/ffmpeg.rs server/src/main.rs
git commit -m "port ffmpeg filter-graph builder"
```

### Task 3.2: Add `trim_clip` (async stream-copy trim)

**Files:**
- Modify: `server/src/ffmpeg.rs`

- [ ] **Step 1: Add the function**

```rust
use std::path::Path;
use color_eyre::{Result, eyre::bail};

/// Trim a remote clip to [start_ms, end_ms] via stream copy into a local segment.
pub async fn trim_clip(url: &str, start_ms: u64, end_ms: u64, out: &Path) -> Result<()> {
    let start = format!("{:.3}", start_ms as f64 / 1000.0);
    let end = format!("{:.3}", end_ms as f64 / 1000.0);
    let status = tokio::process::Command::new("ffmpeg")
        .arg("-ss")
        .arg(&start)
        .arg("-to")
        .arg(&end)
        .arg("-i")
        .arg(url)
        .arg("-c")
        .arg("copy")
        .arg("-avoid_negative_ts")
        .arg("make_zero")
        .arg("-y")
        .arg(out)
        .status()
        .await?;
    if !status.success() {
        bail!("ffmpeg trim failed for {url}");
    }
    Ok(())
}
```

- [ ] **Step 2: Verify + commit**

```bash
cargo check --manifest-path server/Cargo.toml
git add server/src/ffmpeg.rs
git commit -m "add async stream-copy trim"
```

### Task 3.3: Add async `render` with progress

**Files:**
- Modify: `server/src/ffmpeg.rs`

- [ ] **Step 1: Add imports + struct + function**

```rust
use std::{path::PathBuf, process::Stdio, sync::Arc, sync::atomic::{AtomicU64, Ordering}};
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::info;

use crate::types::{ClipInfo, EncodingArgs, ScrollEasing};

pub struct RenderParams {
    pub clips: Vec<ClipInfo>,
    pub output: PathBuf,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub duration_secs: u32,
    pub encoding: EncodingArgs,
    pub text: Option<Text>,
    pub audio_path: Option<PathBuf>,
    pub easing: ScrollEasing,
    /// out_time in microseconds, updated as ffmpeg reports progress
    pub progress: Arc<AtomicU64>,
}
```

- [ ] **Step 2: Add the async render function** (mirrors the old `create_scrolling_video` but async, no indicatif):

```rust
pub async fn render(params: RenderParams) -> Result<()> {
    let RenderParams {
        clips,
        output,
        viewport_width,
        viewport_height,
        duration_secs,
        encoding,
        text,
        audio_path,
        easing,
        progress,
    } = params;

    let total_width: u32 = clips.iter().map(|c| c.output_width).sum();
    if total_width <= viewport_width {
        bail!(
            "Total canvas width ({total_width}) must exceed viewport width ({viewport_width}). \
             Select more clips or a narrower viewport."
        );
    }
    let max_offset = total_width - viewport_width;

    let filter_graph = build_filter_graph(
        &clips, viewport_width, viewport_height, duration_secs, max_offset, text, easing,
    );
    info!("Filter graph:\n{filter_graph}");

    let mut cmd = tokio::process::Command::new("ffmpeg");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    for clip in &clips {
        if clip.is_image {
            cmd.arg("-loop").arg("1");
        } else {
            cmd.arg("-stream_loop").arg("-1");
        }
        cmd.arg("-i").arg(&clip.path);
    }

    let audio_input_index = clips.len();
    if let Some(audio) = &audio_path {
        cmd.arg("-i").arg(audio);
    }

    cmd.arg("-filter_complex").arg(&filter_graph);
    cmd.arg("-map").arg("[out]");
    if audio_path.is_some() {
        cmd.arg("-map").arg(format!("{audio_input_index}:a"));
        cmd.arg("-c:a").arg("aac");
        cmd.arg("-shortest");
    } else {
        cmd.arg("-an");
    }
    cmd.arg("-t").arg(duration_secs.to_string());
    cmd.arg("-c:v").arg(&encoding.codec);
    for arg in &encoding.preset_args {
        cmd.arg(arg);
    }
    cmd.arg(encoding.quality_flag).arg(encoding.quality_value.to_string());
    cmd.arg("-y").arg("-progress").arg("-").arg("-nostats");
    cmd.arg(&output);

    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().unwrap();
    let mut lines = BufReader::new(stdout).lines();
    while let Some(line) = lines.next_line().await? {
        if let Some(rest) = line.strip_prefix("out_time_us=") {
            if let Ok(us) = rest.parse::<u64>() {
                progress.store(us, Ordering::Relaxed);
            }
        }
    }
    let status = child.wait().await?;
    if !status.success() {
        let mut err = child.stderr.take().unwrap();
        let mut buf = String::new();
        use tokio::io::AsyncReadExt;
        err.read_to_string(&mut buf).await?;
        bail!("ffmpeg failed ({status}):\n{buf}");
    }
    Ok(())
}
```

- [ ] **Step 3: Verify + commit**

```bash
cargo check --manifest-path server/Cargo.toml
git add server/src/ffmpeg.rs
git commit -m "async render with progress reporting"
```

### Task 3.4: Add job registry

**Files:**
- Modify: `server/src/jobs.rs` (replace placeholder)

- [ ] **Step 1: Write `server/src/jobs.rs`**

```rust
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub enum JobState { Queued, Running, Done, Failed }

#[derive(Clone, Debug)]
pub struct JobStatus {
    pub state: JobState,
    pub progress: f32,
    pub output_path: Option<PathBuf>,
    pub error: Option<String>,
}

impl JobStatus {
    fn new() -> Self {
        Self { state: JobState::Queued, progress: 0.0, output_path: None, error: None }
    }
}

#[derive(Default)]
pub struct JobRegistry {
    jobs: Mutex<HashMap<Uuid, Arc<Mutex<JobStatus>>>>,
}

impl JobRegistry {
    pub async fn create(&self) -> (Uuid, Arc<Mutex<JobStatus>>) {
        let id = Uuid::new_v4();
        let status = Arc::new(Mutex::new(JobStatus::new()));
        self.jobs.lock().await.insert(id, status.clone());
        (id, status)
    }

    pub async fn get(&self, id: Uuid) -> Option<Arc<Mutex<JobStatus>>> {
        self.jobs.lock().await.get(&id).cloned()
    }
}
```

- [ ] **Step 2: Verify + commit**

```bash
cargo check --manifest-path server/Cargo.toml
git add server/src/jobs.rs
git commit -m "add render job registry"
```

---

# Milestone 4 — Render + Selection + Output Endpoints

### Task 4.1: Render endpoint (trim + dispatch job)

**Files:**
- Modify: `server/src/api.rs`

- [ ] **Step 1: Add DTOs + render handler to `api.rs`**

```rust
use std::{path::PathBuf, sync::{Arc, atomic::{AtomicU64, Ordering}}};
use tokio::sync::Mutex;
use axum::extract::Path;

use crate::{
    ffmpeg::{self, RenderParams},
    jobs::{JobState, JobStatus},
    types::{AspectRatio, ClipInfo, Codec, Effort, EncodingArgs, Quality, ScrollEasing},
};

#[derive(Deserialize)]
pub struct ClipSelection {
    pub id: Uuid,
    pub content_url: String,
    #[serde(default = "default_video")]
    pub file_type: String,       // "video" | "image"
    pub mime_type: String,
    pub width: i32,
    pub height: i32,
    #[serde(default)]
    pub start_ms: u64,
    #[serde(default)]
    pub end_ms: u64,
    #[serde(default)]
    pub performers: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_video() -> String { "video".into() }

#[derive(Deserialize)]
pub struct RenderRequest {
    pub clips: Vec<ClipSelection>,
    #[serde(default = "default_viewport_w")]
    pub viewport_width: u32,
    #[serde(default = "default_viewport_h")]
    pub viewport_height: u32,
    #[serde(default = "default_60")]
    pub duration_secs: u32,
    #[serde(default)]
    pub codec: Codec,
    #[serde(default)]
    pub quality: Quality,
    #[serde(default)]
    pub effort: Effort,
    #[serde(default)]
    pub gpu: bool,
    #[serde(default)]
    pub easing: ScrollEasing,
    #[serde(default)]
    pub text: Option<ffmpeg::Text>,
    pub crop: Option<String>,
    pub song: Option<String>,
}

fn default_viewport_w() -> u32 { 1920 }
fn default_viewport_h() -> u32 { 1080 }
fn default_60() -> u32 { 60 }
```

- [ ] **Step 2: Add the job runner** (a free function that trims, builds `ClipInfo`, renders):

```rust
async fn run_render(
    state: AppState,
    req: RenderRequest,
    status: Arc<Mutex<JobStatus>>,
) {
    let result: color_eyre::Result<PathBuf> = async {
        if req.clips.is_empty() {
            color_eyre::eyre::bail!("select at least one clip");
        }

        let crop_width = req
            .crop
            .as_deref()
            .map(AspectRatio::parse)
            .transpose()?
            .map(|a| a.crop_width(req.viewport_height));

        // 1. Prepare inputs: trim videos with a sub-range to local segments
        let mut clips: Vec<ClipInfo> = Vec::with_capacity(req.clips.len());
        for (i, sel) in req.clips.iter().enumerate() {
            let is_image = sel.file_type == "image";
            let (w, h) = (sel.width as u32, sel.height as u32);

            let mut scaled_w = if h == 0 { req.viewport_width } else {
                (w as u64 * req.viewport_height as u64 / h as u64) as u32
            };
            scaled_w += scaled_w % 2;
            let output_w = crop_width.map(|cw| scaled_w.min(cw)).unwrap_or(scaled_w);

            let path = if !is_image && sel.end_ms > sel.start_ms {
                let seg = state.work_dir.join(format!("seg-{i}.mp4"));
                ffmpeg::trim_clip(&sel.content_url, sel.start_ms, sel.end_ms, &seg).await?;
                seg.to_string_lossy().into_owned()
            } else {
                sel.content_url.clone()
            };

            clips.push(ClipInfo {
                path,
                scaled_width: scaled_w,
                output_width: output_w,
                performers: sel.performers.clone(),
                tags: sel.tags.clone(),
                popularity: 0.0,
                is_image,
            });
        }

        // 2. Optional audio
        let audio_path = if let Some(song) = &req.song {
            Some(crate::song::download_song(song, &state.work_dir).await?)
        } else {
            None
        };

        // 3. Render
        let id = req.clips[0].id;
        let _ = id;
        let output = state.config.output_dir.join(format!("{}.mp4", uuid::Uuid::new_v4()));
        let progress = Arc::new(AtomicU64::new(0));
        let gpu = req.gpu || state.config.gpu;
        ffmpeg::render(RenderParams {
            clips,
            output: output.clone(),
            viewport_width: req.viewport_width,
            viewport_height: req.viewport_height,
            duration_secs: req.duration_secs,
            encoding: EncodingArgs::new(&req.codec, &req.quality, &req.effort, gpu),
            text: req.text,
            audio_path,
            easing: req.easing,
            progress,
        })
        .await?;

        Ok(output)
    }
    .await;

    let mut s = status.lock().await;
    match result {
        Ok(path) => {
            s.state = JobState::Done;
            s.progress = 1.0;
            s.output_path = Some(path);
        }
        Err(e) => {
            s.state = JobState::Failed;
            s.error = Some(format!("{e:#}"));
        }
    }
}
```

> Note: the audio `download_song` is still sync (`std::process`). Wrap it in `tokio::task::spawn_blocking` — see Task 4.5. For now keep the call and add a small async `song.rs` in Task 4.5.

- [ ] **Step 3: Add the `start_render` handler**

```rust
#[derive(Serialize)]
pub struct JobCreated { pub id: Uuid }

async fn start_render(
    State(state): State<AppState>,
    Json(req): Json<RenderRequest>,
) -> ApiResult<Json<JobCreated>> {
    let (id, status) = state.jobs.create().await;
    let job_state = state.clone();
    tokio::spawn(async move {
        {
            let mut s = status.lock().await;
            s.state = JobState::Running;
        }
        run_render(job_state, req, status).await;
    });
    Ok(Json(JobCreated { id }))
}
```

- [ ] **Step 4: Add the status + output handlers**

```rust
#[derive(Serialize)]
pub struct RenderStatus {
    pub state: String,
    pub progress: f32,
    pub output_path: Option<String>,
    pub error: Option<String>,
}

async fn render_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RenderStatus>> {
    let job = state.jobs.get(id).await.ok_or(crate::error::ApiError::NotFound("job not found".into()))?;
    let s = job.lock().await;
    Ok(Json(RenderStatus {
        state: match s.state {
            JobState::Queued => "queued",
            JobState::Running => "running",
            JobState::Done => "done",
            JobState::Failed => "failed",
        }.into(),
        progress: s.progress,
        output_path: s.output_path.as_ref().map(|p| p.to_string_lossy().into_owned()),
        error: s.error.clone(),
    }))
}
```

- [ ] **Step 5: Register routes in `router()`**

```rust
.route("/api/render", post(start_render))
.route("/api/render/{id}", get(render_status))
```

- [ ] **Step 6: Verify + commit**

```bash
cargo check --manifest-path server/Cargo.toml
git add server/src/api.rs
git commit -m "add render + status endpoints"
```

### Task 4.2: Output download endpoint

**Files:**
- Modify: `server/src/api.rs`

- [ ] **Step 1: Add handler**

```rust
use axum::body::Body;
use axum::http::{HeaderValue, header};
use axum::response::IntoResponse;

async fn download_output(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let path = state.config.output_dir.join(format!("{id}.mp4"));
    if !path.exists() {
        return Err(crate::error::ApiError::NotFound("output not found".into()));
    }
    let file = tokio::fs::File::open(&path).await?;
    let stream = tokio_util::io::ReaderStream::new(file);
    let body = Body::from_stream(stream);
    Ok((
        [
            (header::CONTENT_TYPE, HeaderValue::from_static("video/mp4")),
            (header::CONTENT_DISPOSITION, HeaderValue::from_str(&format!("attachment; filename=\"{id}.mp4\"")).unwrap()),
        ],
        body,
    ))
}
```

- [ ] **Step 2: Register route**

```rust
.route("/api/output/{id}", get(download_output))
```

- [ ] **Step 3: Verify + commit**

```bash
cargo check --manifest-path server/Cargo.toml
git add server/src/api.rs
git commit -m "add output download endpoint"
```

### Task 4.3: Thumbnail (poster) endpoint

**Files:**
- Modify: `server/src/api.rs`

- [ ] **Step 1: Add handler** (generates + caches a poster JPEG for a clip; `t` = time in seconds, default 1.0)

```rust
use axum::extract::Query;

#[derive(Deserialize)]
pub struct ThumbQuery { #[serde(default = "default_thumb_t")] pub t: f32 }
fn default_thumb_t() -> f32 { 1.0 }

async fn thumbnail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<ThumbQuery>,
) -> ApiResult<impl IntoResponse> {
    let cache = state.work_dir.join(format!("thumb-{id}-{}.jpg", (q.t * 10.0) as u32));
    if !cache.exists() {
        let url = crate::types::MediaFile::content_url_from_id(id);
        let status = tokio::process::Command::new("ffmpeg")
            .arg("-ss").arg(q.t.to_string())
            .arg("-i").arg(url)
            .arg("-frames:v").arg("1")
            .arg("-vf").arg("scale=320:-2")
            .arg("-y")
            .arg(&cache)
            .status().await?;
        if !status.success() {
            return Err(crate::error::ApiError::Internal("thumbnail generation failed".into()));
        }
    }
    let bytes = tokio::fs::read(&cache).await?;
    Ok((
        [(header::CONTENT_TYPE, HeaderValue::from_static("image/jpeg"))],
        bytes,
    ))
}
```

> Note: `content_url_from_id` needs the content base URL. Add a helper to `types.rs` or pass `state.config.content_url` and build the URL inline (id first-char shard + extension). See Task 4.4.

- [ ] **Step 2: Register route**

```rust
.route("/api/thumb/{id}", get(thumbnail))
```

- [ ] **Step 3: Verify + commit**

```bash
cargo check --manifest-path server/Cargo.toml
git add server/src/api.rs
git commit -m "add thumbnail poster endpoint"
```

### Task 4.4: Thumbnail URL helper in `types.rs`

**Files:**
- Modify: `server/src/types.rs`

- [ ] **Step 1: Add a standalone helper** that builds a CDN URL from a bare id (for the thumbnail endpoint, which receives only an id):

```rust
impl MediaFile {
    /// Build a content URL from a bare id + base, guessing a common extension.
    /// Used by the thumbnail endpoint which only has an id.
    pub fn content_url_from_id(id: uuid::Uuid) -> String {
        // Needs the base url; better signature below.
        unreachable!()
    }
}
```

Replace with the correct version (note: id-only thumbnails can't know the mime/extension reliably, so have the frontend pass `contentUrl` to the thumb endpoint instead — see the correction below):

```rust
// types.rs — add nothing new here; instead change the thumbnail handler to
// accept the full content_url (Task 4.3 correction):
```

- [ ] **Step 2: Correct the thumbnail handler** to receive the content URL rather than guessing it (frontend already has `content_url`):

```rust
#[derive(Deserialize)]
pub struct ThumbQuery {
    pub url: String,                    // full CDN content URL
    #[serde(default = "default_thumb_t")]
    pub t: f32,
}

async fn thumbnail(
    State(state): State<AppState>,
    Query(q): Query<ThumbQuery>,
) -> ApiResult<impl IntoResponse> {
    // derive a cache key from the url
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    q.url.hash(&mut h);
    let key = h.finish();
    let cache = state.work_dir.join(format!("thumb-{key}-{}.jpg", (q.t * 10.0) as u32));
    if !cache.exists() {
        let status = tokio::process::Command::new("ffmpeg")
            .arg("-ss").arg(q.t.to_string())
            .arg("-i").arg(&q.url)
            .arg("-frames:v").arg("1")
            .arg("-vf").arg("scale=320:-2")
            .arg("-y")
            .arg(&cache)
            .status().await?;
        if !status.success() {
            return Err(crate::error::ApiError::Internal("thumbnail generation failed".into()));
        }
    }
    let bytes = tokio::fs::read(&cache).await?;
    Ok(([(header::CONTENT_TYPE, HeaderValue::from_static("image/jpeg"))], bytes))
}
```

And register `GET /api/thumb` (no path param) instead of `/api/thumb/{id}`.

- [ ] **Step 3: Verify + commit**

```bash
cargo check --manifest-path server/Cargo.toml
git add server/src/api.rs server/src/types.rs
git commit -m "thumbnail endpoint takes content url"
```

### Task 4.5: Async song download wrapper

**Files:**
- Create: `server/src/song.rs`
- Modify: `server/src/main.rs` (add `mod song;`)

- [ ] **Step 1: Port `src/song.rs` → `server/src/song.rs`**, remove the `indicatif` spinner + `crate::progress_hidden()` usage, and add an async wrapper:

```rust
use std::path::{Path, PathBuf};
use color_eyre::Result;

/// Download audio (blocking) off the async runtime.
pub async fn download_song(url: &str, dest_dir: &Path) -> Result<PathBuf> {
    let url = url.to_string();
    let dest = dest_dir.to_path_buf();
    tokio::task::spawn_blocking(move || download_song_blocking(&url, &dest))
        .await
        .map_err(|e| color_eyre::eyre::eyre!("song download task panicked: {e}"))?
}

fn download_song_blocking(url: &str, dest_dir: &Path) -> Result<PathBuf> {
    // existing yt-dlp logic verbatim (minus the spinner), returning dest_dir.join("song.m4a")
    // ... (copy from src/song.rs)
}
```

- [ ] **Step 2: Update `api.rs` `run_render`** to call `crate::song::download_song(song, &state.work_dir).await`.

- [ ] **Step 3: Verify + commit**

```bash
cargo check --manifest-path server/Cargo.toml
git add server/src/song.rs server/src/api.rs server/src/main.rs
git commit -m "async song download wrapper"
```

---

# Milestone 5 — Frontend (SvelteKit Wizard)

### Task 5.1: Types + API client

**Files:**
- Create: `web/src/lib/types.ts`
- Create: `web/src/lib/api.ts`
- Create: `web/vite.config.ts` (or modify scaffold's)

- [ ] **Step 1: Write `web/src/lib/types.ts`**

```ts
export interface ClipCandidate {
  id: string;
  content_url: string;
  file_type: 'video' | 'image';
  width: number;
  height: number;
  duration_ms: number;
  tags: string[];
  performers: string[];
  popularity: number;
}

export type Orientation = 'portrait' | 'landscape' | 'square' | 'any';
export type Codec = 'x264' | 'hevc' | 'av1';
export type Quality = 'low' | 'medium' | 'high' | 'veryhigh';
export type Effort = 'low' | 'medium' | 'high';
export type Easing = 'linear' | 'ease';

export interface ClipSelection {
  id: string;
  content_url: string;
  file_type: 'video' | 'image';
  mime_type: string;
  width: number;
  height: number;
  start_ms: number;
  end_ms: number;
  performers: string[];
  tags: string[];
}

export interface RenderRequest {
  clips: ClipSelection[];
  viewport_width: number;
  viewport_height: number;
  duration_secs: number;
  codec: Codec;
  quality: Quality;
  effort: Effort;
  gpu: boolean;
  easing: Easing;
  text: 'performers' | 'tags' | null;
  crop: string | null;
  song: string | null;
}

export interface RenderStatus {
  state: 'queued' | 'running' | 'done' | 'failed';
  progress: number;
  output_path: string | null;
  error: string | null;
}
```

- [ ] **Step 2: Write `web/src/lib/api.ts`**

```ts
import type { ClipCandidate, RenderRequest, RenderStatus } from './types';

async function req<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    headers: { 'content-type': 'application/json' },
    ...init,
  });
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(body.error ?? res.statusText);
  }
  return res.json() as Promise<T>;
}

export const api = {
  tags: (q: string) => req<string[]>(`/api/tags?query=${encodeURIComponent(q)}`),
  people: (q: string) => req<string[]>(`/api/people?query=${encodeURIComponent(q)}`),
  pool: (body: unknown) =>
    req<ClipCandidate[]>('/api/pool', { method: 'POST', body: JSON.stringify(body) }),
  render: (body: RenderRequest) =>
    req<{ id: string }>('/api/render', { method: 'POST', body: JSON.stringify(body) }),
  status: (id: string) => req<RenderStatus>(`/api/render/${id}`),
  outputUrl: (id: string) => `/api/output/${id}`,
  thumbUrl: (url: string, t = 1) => `/api/thumb?url=${encodeURIComponent(url)}&t=${t}`,
};
```

- [ ] **Step 3: Configure the dev proxy in `web/vite.config.ts`**

```ts
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    proxy: { '/api': { target: 'http://127.0.0.1:3001', changeOrigin: true } },
  },
});
```

- [ ] **Step 4: Verify + commit**

```bash
cd web && npm run check
git add web/src/lib web/vite.config.ts
git commit -m "frontend types + api client"
```

### Task 5.2: FilterForm component

**Files:**
- Create: `web/src/lib/components/FilterForm.svelte`

- [ ] **Step 1: Write `FilterForm.svelte`** (Svelte 5 runes). Emits a `filters` object + `generate` event:

```svelte
<script lang="ts">
  import type { Orientation } from '$lib/types';

  let tags = $state<string[]>([]);
  let people = $state<string[]>([]);
  let tagInput = $state('');
  let personInput = $state('');
  let orientation = $state<Orientation>('portrait');
  let withImages = $state(false);
  let maxDuration = $state(30);
  let poolSize = $state(50);

  let { onGenerate }: { onGenerate: (f: Filters) => void } = $props();

  export type Filters = {
    tags: string[]; people: string[]; orientation: Orientation;
    with_images: boolean; max_clip_duration_ms: number; pool_size: number;
  };

  function addTag() {
    const v = tagInput.trim();
    if (v && !tags.includes(v)) tags = [...tags, v];
    tagInput = '';
  }
  function addPerson() {
    const v = personInput.trim();
    if (v && !people.includes(v)) people = [...people, v];
    personInput = '';
  }
</script>

<div class="filters">
  <label>Orientation
    <select bind:value={orientation}>
      <option value="portrait">Portrait</option>
      <option value="landscape">Landscape</option>
      <option value="square">Square</option>
      <option value="any">Any</option>
    </select>
  </label>

  <label>Tags (comma/enter separated)
    <input bind:value={tagInput} onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), addTag())} />
    <span class="chips">{#each tags as t}<span class="chip">{t}<button onclick={() => tags = tags.filter(x => x !== t)}>×</button></span>{/each}</span>
  </label>

  <label>People
    <input bind:value={personInput} onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), addPerson())} />
    <span class="chips">{#each people as p}<span class="chip">{p}<button onclick={() => people = people.filter(x => x !== p)}>×</button></span>{/each}</span>
  </label>

  <label><input type="checkbox" bind:checked={withImages} /> Include images</label>
  <label>Max clip duration (s) <input type="number" min="1" max="300" bind:value={maxDuration} /></label>
  <label>Pool size <input type="number" min="5" max="200" bind:value={poolSize} /></label>

  <button onclick={() => onGenerate({
    tags, people, orientation, with_images: withImages,
    max_clip_duration_ms: maxDuration * 1000, pool_size: poolSize,
  })}>
    Generate pool
  </button>
</div>
```

> Note: `onkeydown` inline handlers with tuple/`&&` expressions in Svelte 5 need the value to be a function; write `onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); addTag(); } }}` to be safe.

- [ ] **Step 2: Verify + commit**

```bash
cd web && npm run check
git add web/src/lib/components/FilterForm.svelte
git commit -m "filter form component"
```

### Task 5.3: ClipCard with time-range trim (the core component)

**Files:**
- Create: `web/src/lib/components/ClipCard.svelte`

- [ ] **Step 1: Write `ClipCard.svelte`**

```svelte
<script lang="ts">
  import type { ClipCandidate } from '$lib/types';

  let { clip, selected }: { clip: ClipCandidate; selected: boolean } = $props();

  let startMs = $state(0);
  let endMs = $state(clip.duration_ms || 0);
  let playing = $state(false);
  let video: HTMLVideoElement | undefined = $state();

  $effect(() => {
    if (startMs >= endMs) startMs = Math.max(0, endMs - 500);
  });

  function playSegment() {
    if (!video) return;
    video.currentTime = startMs / 1000;
    void video.play();
    playing = true;
  }

  function onTimeUpdate() {
    if (video && video.currentTime * 1000 >= endMs) {
      video.pause();
      playing = false;
    }
  }

  function onEnded() { playing = false; }

  const isVideo = clip.file_type === 'video';
  const fmt = (ms: number) => `${(ms / 1000).toFixed(1)}s`;
</script>

<div class="card" class:selected>
  <div class="media">
    {#if isVideo}
      <video
        bind:this={video}
        src={clip.content_url}
        preload="metadata"
        muted
        playsinline
        ontimeupdate={onTimeUpdate}
        onended={onEnded}
      ></video>
    {:else}
      <img src={clip.content_url} alt="" />
    {/if}
    {#if isVideo && !playing}
      <button class="play" onclick={playSegment}>▶</button>
    {/if}
  </div>

  <div class="row">
    <span class="title">{clip.performers.join(', ') || clip.tags.slice(0, 3).join(', ') || 'untitled'}</span>
    <span class="popularity">★ {clip.popularity.toFixed(1)}</span>
  </div>

  {#if isVideo}
    <div class="trim">
      <label>Start <input type="range" min="0" max={clip.duration_ms} step="100" bind:value={startMs} /></label>
      <label>End <input type="range" min="0" max={clip.duration_ms} step="100" bind:value={endMs} /></label>
      <div class="times">{fmt(startMs)} → {fmt(endMs)}</div>
    </div>
  {/if}

  <div class="tags">{clip.tags.join(' · ')}</div>
</div>

<style>
  .card { border: 1px solid #333; border-radius: 8px; padding: 8px; display: flex; flex-direction: column; gap: 6px; }
  .card.selected { border-color: #4caf50; box-shadow: 0 0 0 2px #4caf5066; }
  .media { position: relative; aspect-ratio: 9/16; background: #111; border-radius: 6px; overflow: hidden; }
  video, img { width: 100%; height: 100%; object-fit: contain; }
  .play { position: absolute; inset: 0; margin: auto; width: 48px; height: 48px; border-radius: 50%; border: none; background: #000a; color: #fff; font-size: 20px; cursor: pointer; }
  .row { display: flex; justify-content: space-between; font-size: 13px; }
  .title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .popularity { color: #f9a825; white-space: nowrap; }
  .trim { display: flex; flex-direction: column; gap: 4px; font-size: 12px; }
  .trim input { width: 100%; }
  .times { color: #888; }
  .tags { font-size: 11px; color: #aaa; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
```

- [ ] **Step 2: Verify + commit**

```bash
cd web && npm run check
git add web/src/lib/components/ClipCard.svelte
git commit -m "clip card with time-range trim"
```

### Task 5.4: ClipGrid selection

**Files:**
- Create: `web/src/lib/components/ClipGrid.svelte`

- [ ] **Step 1: Write `ClipGrid.svelte`** — renders a responsive grid of `ClipCard`s, tracks which ids are selected:

```svelte
<script lang="ts">
  import type { ClipCandidate } from '$lib/types';
  import ClipCard from './ClipCard.svelte';

  let { clips }: { clips: ClipCandidate[] } = $props();
  let selected = $state<Set<string>>(new Set());

  function toggle(id: string) {
    selected = new Set(selected);
    if (selected.has(id)) selected.delete(id);
    else selected.add(id);
    selected = selected;
  }
</script>

<div class="grid">
  {#each clips as clip (clip.id)}
    <button class="cardwrap" onclick={() => toggle(clip.id)}>
      <ClipCard {clip} selected={selected.has(clip.id)} />
    </button>
  {/each}
</div>

<style>
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 12px; }
  .cardwrap { background: none; border: none; padding: 0; text-align: left; cursor: pointer; }
</style>
```

> Note: the `selected`/`toggle` state must be lifted to the page so the render step can read it. Expose it via `$bindable` or a callback. Simplest: lift selection state into `+page.svelte` and pass `selected` + `onToggle` into `ClipGrid`/`ClipCard` as props. Adjust the component to receive `selected: Set<string>` and `onToggle: (id: string) => void` rather than owning the set.

- [ ] **Step 2: Verify + commit**

```bash
cd web && npm run check
git add web/src/lib/components/ClipGrid.svelte
git commit -m "clip grid selection"
```

### Task 5.5: RenderForm + RenderProgress

**Files:**
- Create: `web/src/lib/components/RenderForm.svelte`
- Create: `web/src/lib/components/RenderProgress.svelte`

- [ ] **Step 1: Write `RenderForm.svelte`** — codec/quality/effort/easing/text/crop/song + a "Render" button that emits the assembled `RenderRequest`:

```svelte
<script lang="ts">
  import type { Codec, Effort, Easing, Quality } from '$lib/types';

  let codec = $state<Codec>('x264');
  let quality = $state<Quality>('medium');
  let effort = $state<Effort>('medium');
  let easing = $state<Easing>('linear');
  let gpu = $state(false);
  let text = $state<'performers' | 'tags' | 'none'>('none');
  let crop = $state('');
  let song = $state('');
  let duration = $state(60);

  let { onRender }: { onRender: (o: Record<string, unknown>) => void } = $props();

  function submit() {
    onRender({
      codec, quality, effort, easing, gpu,
      text: text === 'none' ? null : text,
      crop: crop || null,
      song: song || null,
      duration_secs: duration,
      viewport_width: 1080,
      viewport_height: 1920,
    });
  }
</script>

<div class="renderform">
  <label>Duration <input type="number" bind:value={duration} min="5" max="120" />s</label>
  <label>Codec
    <select bind:value={codec}><option value="x264">H.264</option><option value="hevc">HEVC</option><option value="av1">AV1</option></select>
  </label>
  <label>Quality
    <select bind:value={quality}><option value="low">Low</option><option value="medium">Medium</option><option value="high">High</option><option value="veryhigh">Very High</option></select>
  </label>
  <label>Effort
    <select bind:value={effort}><option value="low">Low</option><option value="medium">Medium</option><option value="high">High</option></select>
  </label>
  <label>Easing
    <select bind:value={easing}><option value="linear">Linear</option><option value="ease">Ease</option></select>
  </label>
  <label><input type="checkbox" bind:checked={gpu} /> GPU encoding</label>
  <label>Overlay text
    <select bind:value={text}><option value="none">None</option><option value="performers">Performers</option><option value="tags">Tags</option></select>
  </label>
  <label>Crop aspect (e.g. 9:16) <input bind:value={crop} placeholder="optional" /></label>
  <label>Song URL <input bind:value={song} placeholder="optional" /></label>
  <button onclick={submit}>Render</button>
</div>
```

> Note: **viewport is portrait for RedGifs.** Default `viewport_width=1080`, `viewport_height=1920` (9:16). The old tool defaulted to landscape 1920×1080; RedGifs compilations are vertical, so the wizard should default to 1080×1920. Adjust `default_viewport_w/h` in `api.rs` Task 4.1 to `1080`/`1920` accordingly (or leave backend defaults and always send explicit values from the form — **send explicit values**).

- [ ] **Step 2: Write `RenderProgress.svelte`** — polls `/api/render/{id}` every 1s, shows progress bar, and a download link on completion:

```svelte
<script lang="ts">
  import { api } from '$lib/api';
  import type { RenderStatus } from '$lib/types';

  let { id }: { id: string } = $props();
  let status = $state<RenderStatus | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    const timer = setInterval(async () => {
      try {
        status = await api.status(id);
        if (status && (status.state === 'done' || status.state === 'failed')) clearInterval(timer);
      } catch (e) {
        error = String(e);
        clearInterval(timer);
      }
    }, 1000);
    return () => clearInterval(timer);
  });

  const pct = $derived(Math.round((status?.progress ?? 0) * 100));
</script>

{#if error}
  <div class="error">{error}</div>
{:else}
  <div class="progress">
    <div class="bar" style={`width:${pct}%`}></div>
    <span>{status?.state ?? 'queued'} — {pct}%</span>
    {#if status?.state === 'failed'}<div class="error">{status.error}</div>{/if}
    {#if status?.state === 'done' && status.output_path}
      <a href={api.outputUrl(id)} download>Download compilation</a>
    {/if}
  </div>
{/if}

<style>
  .progress { margin: 16px 0; }
  .bar { height: 8px; background: #4caf50; border-radius: 4px; transition: width .3s; }
  .error { color: #ef5350; }
</style>
```

- [ ] **Step 3: Verify + commit**

```bash
cd web && npm run check
git add web/src/lib/components/RenderForm.svelte web/src/lib/components/RenderProgress.svelte
git commit -m "render form + progress components"
```

### Task 5.6: Wizard page (`+page.svelte`)

**Files:**
- Create: `web/src/routes/+page.svelte`

- [ ] **Step 1: Write `+page.svelte`** (orchestrates the 3 steps):

```svelte
<script lang="ts">
  import { api } from '$lib/api';
  import type { ClipCandidate, ClipSelection, RenderRequest } from '$lib/types';
  import FilterForm from '$lib/components/FilterForm.svelte';
  import ClipGrid from '$lib/components/ClipGrid.svelte';
  import RenderForm from '$lib/components/RenderForm.svelte';
  import RenderProgress from '$lib/components/RenderProgress.svelte';

  let step = $state<'filters' | 'pool' | 'render'>('filters');
  let clips = $state<ClipCandidate[]>([]);
  let selected = $state<Set<string>>(new Set());
  let loading = $state(false);
  let error = $state<string | null>(null);
  let jobId = $state<string | null>(null);

  function toggle(id: string) {
    selected = new Set(selected);
    selected.has(id) ? selected.delete(id) : selected.add(id);
    selected = selected;
  }

  async function onGenerate(filters: Record<string, unknown>) {
    loading = true; error = null;
    try {
      clips = await api.pool(filters);
      step = 'pool';
    } catch (e) { error = String(e); }
    finally { loading = false; }
  }

  async function onRender(opts: Record<string, unknown>) {
    const chosen = clips.filter((c) => selected.has(c.id));
    const selections: ClipSelection[] = chosen.map((c) => ({
      id: c.id, content_url: c.content_url, file_type: c.file_type,
      mime_type: c.file_type === 'video' ? 'video/mp4' : 'image/jpeg',
      width: c.width, height: c.height,
      start_ms: 0, end_ms: c.duration_ms,
      performers: c.performers, tags: c.tags,
    }));
    const body = { ...opts, clips: selections } as RenderRequest;
    loading = true; error = null;
    try {
      const { id } = await api.render(body);
      jobId = id; step = 'render';
    } catch (e) { error = String(e); }
    finally { loading = false; }
  }
</script>

<main>
  <h1>Scrolling Compilation Maker</h1>
  {#if error}<div class="error">{error}</div>{/if}

  {#if step === 'filters'}
    <FilterForm {onGenerate} />
  {:else if step === 'pool'}
    <button onclick={() => { step = 'filters'; }}>← Back to filters</button>
    {#if loading}<p>Loading…</p>{/if}
    <ClipGrid {clips} {selected} {toggle} />
    <button disabled={selected.size === 0} onclick={() => { step = 'render'; }}>
      Continue with {selected.size} clip{selected.size === 1 ? '' : 's'}
    </button>
  {:else}
    <RenderForm {onRender} />
    {#if jobId}<RenderProgress id={jobId} />{/if}
  {/if}
</main>

<style>
  main { max-width: 1400px; margin: 0 auto; padding: 16px; font-family: system-ui, sans-serif; }
  .error { color: #ef5350; }
  button { padding: 8px 14px; margin: 8px 0; }
  button:disabled { opacity: .4; }
</style>
```

> Important: `ClipGrid` must receive `selected` + `toggle` as props (not own the state) — reflect that in the ClipGrid component signature from Task 5.4. The `+page.svelte` above already passes them.

- [ ] **Step 2: Verify the full app builds**

```bash
cd web && npm run build
```
Expected: build succeeds with no Svelte errors.

- [ ] **Step 3: Commit**

```bash
git add web/src/routes/+page.svelte
git commit -m "wizard page wiring filters/pool/render"
```

---

# Milestone 6 — Integration Test, Cleanup, Docs

### Task 6.1: End-to-end smoke test

- [ ] **Step 1: Start the backend**

```bash
cargo build --manifest-path server/Cargo.toml
OUTPUT_DIR=./server/output ./server/target/debug/scm-server
```

- [ ] **Step 2: Start the frontend**

```bash
cd web && npm run dev
```

- [ ] **Step 3: Run the flow in a browser at http://localhost:5173**

1. Choose orientation/tags/people → "Generate pool" → grid appears with poster previews.
2. Pick several clips, drag their start/end sliders, hit "▶" to preview a segment.
3. "Continue" → set duration 60, codec H.264, medium → "Render".
4. Watch progress bar reach 100%, click "Download compilation".
5. Open the `.mp4` and confirm it's a 60s vertical scroll with the selected segments looping.

- [ ] **Step 4: Verify output**

```bash
ffprobe -v error -show_entries format=duration -of csv=p=0 server/output/<uuid>.mp4
```
Expected: ~60.0

- [ ] **Step 5: Commit any fixes** (only if behavior was wrong)

```bash
git add -A
git commit -m "fix issues found in smoke test"
```

### Task 6.2: Remove the old CLI/TUI

**Files:**
- Delete: `src/`, `Cargo.toml` (root), `Cargo.lock` (root), `data/` (if unused)

- [ ] **Step 1: Confirm the server no longer references root files** (data wordlists were dropped in Task 1.1; verify no `include_str!("../data")` remains in `server/`).

Run: `search_files('data/', target='content', path='server')` → Expected: no matches.

- [ ] **Step 2: Delete the old crate**

```bash
git rm -r src Cargo.toml Cargo.lock data
```

- [ ] **Step 3: Verify the repo builds from the top level**

```bash
cargo build --manifest-path server/Cargo.toml
cd web && npm run build
```
Expected: both succeed.

- [ ] **Step 4: Commit**

```bash
git commit -m "remove legacy CLI/TUI"
```

### Task 6.3: README + run docs

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write `README.md`** covering: prerequisites (ffmpeg, yt-dlp optional, node, rust), env vars (`ALEXANDRIA_API_URL`, `CONTENT_URL`, `BIND_ADDR`, `OUTPUT_DIR`, `GPU`), how to run backend + frontend, and the 3-step workflow.

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "add README"
```

---

## Verification Checklist (final)

- [ ] `cargo build --manifest-path server/Cargo.toml` clean
- [ ] `cargo clippy --manifest-path server/Cargo.toml` no warnings that matter
- [ ] `cd web && npm run check` clean
- [ ] `cd web && npm run build` clean
- [ ] Full 60s vertical render produced and downloads via the UI
- [ ] Per-clip start/end trimming reflected in the output
- [ ] Old CLI/TUI removed; repo is `server/` + `web/` + `README.md`

## Out of Scope / Future Work

- Frame-accurate trimming (re-encode instead of `-c copy`)
- Selection/pool persistence across reloads
- RedGifs/Reddit auto-posting (user handles manually)
- Background queue with cancellation
- Serving the built frontend from the axum server in production (currently dev uses the Vite proxy; for prod either `vite build` + `tower-http` `ServeDir`, or a reverse proxy)
