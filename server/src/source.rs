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
