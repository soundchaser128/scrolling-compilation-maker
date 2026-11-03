use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tracing::Instrument;
use uuid::Uuid;

use crate::{
    error::ApiResult,
    source::{MediaSource, PoolParams},
    state::AppState,
    types::{FileType, MediaFile, Orientation},
};

#[derive(Serialize)]
pub struct ClipCandidate {
    pub id: Uuid,
    pub content_url: String,
    pub title: Option<String>,
    pub alexandria_url: String,
    pub file_type: String,
    pub mime_type: String,
    pub width: i32,
    pub height: i32,
    pub duration_ms: u64,
    pub tags: Vec<String>,
    pub performers: Vec<String>,
    pub popularity: f64,
}

fn candidate(m: &MediaFile, content_base: &str) -> ClipCandidate {
    ClipCandidate {
        id: m.id,
        content_url: m.content_url(content_base),
        title: m.title.clone(),
        alexandria_url: m.alexandria_url(),
        file_type: match m.file_type {
            FileType::Image => "image".into(),
            _ => "video".into(),
        },
        mime_type: m.mime_type.clone(),
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

fn default_max_duration() -> u64 {
    30_000
}
fn default_pool_size() -> usize {
    50
}

#[derive(Deserialize)]
pub struct AutocompleteQuery {
    pub query: Option<String>,
}

#[tracing::instrument(skip_all, fields(pool_size = req.pool_size, max_clip_duration_ms = req.max_clip_duration_ms, with_images = req.with_images))]
async fn create_pool(
    State(state): State<AppState>,
    Json(req): Json<PoolRequest>,
) -> ApiResult<Json<Vec<ClipCandidate>>> {
    if !(1..=200).contains(&req.pool_size) || req.max_clip_duration_ms == 0 {
        return Err(crate::error::ApiError::BadRequest(
            "pool size must be 1-200 and duration must be positive".into(),
        ));
    }
    let seed = req.seed.unwrap_or_else(rand::random);
    let files = state
        .source
        .fetch_pool(PoolParams {
            pool_size: req.pool_size,
            max_clip_duration_ms: req.max_clip_duration_ms,
            seed,
            orientation: req.orientation,
            tags: req.tags,
            people: req.people,
            with_images: req.with_images,
        })
        .await?;
    let mut candidates: Vec<ClipCandidate> = files
        .iter()
        .map(|m| candidate(m, &state.config.content_url))
        .collect();
    candidates.sort_by(|c, d| d.popularity.total_cmp(&c.popularity));

    tracing::info!(count = candidates.len(), "created media pool");
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
        .route("/api/render", post(start_render))
        .route("/api/render/{id}", get(render_status))
        .route("/api/output/{id}", get(download_output))
        .route("/api/summary/{id}", get(download_summary))
        .with_state(state)
}

use axum::extract::Path;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::sync::Mutex;

use crate::{
    ffmpeg::{self, RenderParams},
    jobs::{JobState, JobStatus},
    types::{AspectRatio, ClipInfo, Codec, Effort, EncodingArgs, Quality, ScrollEasing},
};

#[derive(Deserialize)]
#[allow(unused)]
pub struct ClipSelection {
    pub id: Uuid,
    pub content_url: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub alexandria_url: String,
    #[serde(default = "default_video")]
    pub file_type: String, // "video" | "image"
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

fn default_video() -> String {
    "video".into()
}

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

fn default_viewport_w() -> u32 {
    1920
}
fn default_viewport_h() -> u32 {
    1080
}
fn default_60() -> u32 {
    60
}

#[tracing::instrument(skip_all, fields(job_id = %id, clip_count = req.clips.len(), duration_secs = req.duration_secs))]
async fn run_render(
    state: AppState,
    req: RenderRequest,
    status: Arc<Mutex<JobStatus>>,
    id: Uuid,
    progress: Arc<AtomicU64>,
) {
    tracing::info!("render started");
    let result: color_eyre::Result<PathBuf> = async {
        if req.clips.is_empty() {
            color_eyre::eyre::bail!("select at least one clip");
        }

        let crop_width = req
            .crop
            .as_deref()
            .map(AspectRatio::parse)
            .transpose()
            .map_err(|e| color_eyre::eyre::eyre!(e))?
            .map(|a| a.crop_width(req.viewport_height));

        let job_dir = state.work_dir.join(id.to_string());
        tokio::fs::create_dir_all(&job_dir).await?;
        // 1. Prepare inputs: trim videos with a sub-range to local segments
        let mut clips: Vec<ClipInfo> = Vec::with_capacity(req.clips.len());
        for (i, sel) in req.clips.iter().enumerate() {
            let is_image = sel.file_type == "image";
            let (w, h) = (sel.width as u32, sel.height as u32);

            let mut scaled_w = if h == 0 {
                req.viewport_width
            } else {
                (w as u64 * req.viewport_height as u64 / h as u64) as u32
            };
            scaled_w += scaled_w % 2;
            let output_w = crop_width.map(|cw| scaled_w.min(cw)).unwrap_or(scaled_w);

            let path = if !is_image && sel.end_ms > sel.start_ms {
                let seg = job_dir.join(format!("seg-{i}.mkv"));
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
            Some(crate::song::download_song(song, &job_dir).await?)
        } else {
            None
        };

        // 3. Render
        let output = state.config.output_dir.join(format!("{id}.mp4"));
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

        tokio::fs::write(
            state.config.output_dir.join(format!("{id}.md")),
            markdown_summary(&req.clips),
        )
        .await?;
        Ok(output)
    }
    .await;

    let mut s = status.lock().await;
    match result {
        Ok(path) => {
            tracing::info!("render completed");
            s.state = JobState::Done;
            s.progress = 1.0;
            s.output_path = Some(path);
        }
        Err(e) => {
            tracing::error!(error = %e, "render failed");
            s.state = JobState::Failed;
            s.error = Some(format!("{e:#}"));
        }
    }
}

#[derive(Serialize)]
pub struct JobCreated {
    pub id: Uuid,
}

#[tracing::instrument(skip_all, fields(clip_count = req.clips.len(), duration_secs = req.duration_secs))]
async fn start_render(
    State(state): State<AppState>,
    Json(req): Json<RenderRequest>,
) -> ApiResult<Json<JobCreated>> {
    if req.clips.len() < 2
        || !(1..=120).contains(&req.duration_secs)
        || req.viewport_width == 0
        || req.viewport_height == 0
        || req.viewport_width > 3840
        || req.viewport_height > 3840
        || req.clips.iter().any(|c| {
            c.width <= 0
                || c.height <= 0
                || !matches!(c.file_type.as_str(), "video" | "image")
                || (c.file_type == "video" && c.end_ms <= c.start_ms)
        })
    {
        return Err(crate::error::ApiError::BadRequest(
            "invalid render settings or clip range".into(),
        ));
    }
    let (id, status) = state.jobs.create().await;
    tracing::info!(job_id = %id, "render queued");
    let progress = Arc::new(AtomicU64::new(0));
    {
        let mut s = status.lock().await;
        s.progress_us = progress.clone();
        s.duration_secs = req.duration_secs;
    }
    let job_state = state.clone();
    tokio::spawn(
        async move {
            {
                let mut s = status.lock().await;
                s.state = JobState::Running;
            }
            run_render(job_state, req, status, id, progress).await;
        }
        .instrument(tracing::Span::current()),
    );
    Ok(Json(JobCreated { id }))
}

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
    let job = state
        .jobs
        .get(id)
        .await
        .ok_or(crate::error::ApiError::NotFound("job not found".into()))?;
    let s = job.lock().await;
    Ok(Json(RenderStatus {
        state: match s.state {
            JobState::Queued => "queued",
            JobState::Running => "running",
            JobState::Done => "done",
            JobState::Failed => "failed",
        }
        .into(),
        progress: if matches!(s.state, JobState::Done) {
            1.0
        } else {
            (s.progress_us.load(Ordering::Relaxed) as f32 / (s.duration_secs as f32 * 1_000_000.0))
                .clamp(0.0, 0.99)
        },
        output_path: s
            .output_path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned()),
        error: s.error.clone(),
    }))
}

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
            (
                header::CONTENT_DISPOSITION,
                HeaderValue::from_str(&format!("attachment; filename=\"{id}.mp4\"")).unwrap(),
            ),
        ],
        body,
    ))
}

fn markdown_summary(clips: &[ClipSelection]) -> String {
    fn escape(value: &str) -> String {
        value
            .chars()
            .flat_map(|c| match c {
                '\n' | '\r' => " ".to_string().chars().collect::<Vec<_>>(),
                '\\' | '*' | '_' | '[' | ']' | '`' | '<' | '>' | '#' => vec!['\\', c],
                _ => vec![c],
            })
            .collect()
    }
    let mut summary = String::from("## Clips\n\n");
    for (i, clip) in clips.iter().enumerate() {
        let people = if clip.performers.is_empty() {
            "Unknown performer".into()
        } else {
            clip.performers
                .iter()
                .map(|p| escape(p))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let title = clip
            .title
            .as_deref()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or("Untitled");
        let mut chars = title.chars();
        let truncated: String = chars.by_ref().take(100).collect();
        let title = if chars.next().is_some() {
            format!("{}…", truncated.trim_end())
        } else {
            truncated
        };
        let url = clip
            .alexandria_url
            .replace('>', "%3E")
            .replace('<', "%3C")
            .replace(' ', "%20")
            .replace('\n', "")
            .replace('\r', "");
        if url.is_empty() {
            summary.push_str(&format!("{}. {} — {}\n", i + 1, escape(&title), people));
        } else {
            summary.push_str(&format!(
                "{}. [{}](<{}>) — {}\n",
                i + 1,
                escape(&title),
                url,
                people
            ));
        }
    }
    summary
}

async fn download_summary(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let path = state.config.output_dir.join(format!("{id}.md"));
    if !path.exists() {
        return Err(crate::error::ApiError::NotFound("summary not found".into()));
    }
    let bytes = tokio::fs::read(path).await?;
    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/markdown; charset=utf-8"),
            ),
            (
                header::CONTENT_DISPOSITION,
                HeaderValue::from_str(&format!("attachment; filename=\"{id}.md\"")).unwrap(),
            ),
        ],
        bytes,
    ))
}
