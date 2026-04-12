use color_eyre::Result;
use std::time::Duration;

use crate::types::{MediaFile, Orientation};

pub mod alexandria;
pub mod stash;

pub struct FetchVideosParams<'a> {
    pub max_clip_duration: Duration,
    pub desired_count: usize,
    pub seed: f64,
    pub orientation: Orientation,
    pub tags: &'a [String],
    pub people: &'a [String],
    pub with_images: bool,
}

pub trait MediaSource {
    fn fetch(&self, params: FetchVideosParams<'_>) -> Result<Vec<MediaFile>>;

    fn fetch_people(&self, prefix: Option<&str>) -> Result<Vec<String>>;

    fn fetch_tags(&self, prefix: Option<&str>) -> Result<Vec<String>>;
}
