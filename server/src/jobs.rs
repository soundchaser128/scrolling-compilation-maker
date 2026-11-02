use std::sync::atomic::AtomicU64;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub enum JobState {
    Queued,
    Running,
    Done,
    Failed,
}

#[derive(Clone, Debug)]
pub struct JobStatus {
    pub state: JobState,
    pub progress: f32,
    pub progress_us: Arc<AtomicU64>,
    pub duration_secs: u32,
    pub output_path: Option<PathBuf>,
    pub error: Option<String>,
}

impl JobStatus {
    fn new() -> Self {
        Self {
            state: JobState::Queued,
            progress: 0.0,
            progress_us: Arc::new(AtomicU64::new(0)),
            duration_secs: 60,
            output_path: None,
            error: None,
        }
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
