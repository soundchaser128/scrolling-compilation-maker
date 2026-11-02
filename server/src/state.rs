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
