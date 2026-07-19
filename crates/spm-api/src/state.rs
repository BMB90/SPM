use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use spm_storage::Storage;
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CaptureStatus {
    Running,
    Complete,
    Failed { error: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptureStatusEvent {
    pub session_id: Uuid,
    #[serde(flatten)]
    pub status: CaptureStatus,
}

#[derive(Clone)]
pub struct AppState {
    pub storage: Storage,
    pub capture_status: Arc<Mutex<HashMap<Uuid, CaptureStatus>>>,
    pub events: broadcast::Sender<CaptureStatusEvent>,
}

impl AppState {
    pub fn new(storage: Storage) -> Self {
        let (events, _rx) = broadcast::channel(256);
        Self { storage, capture_status: Arc::new(Mutex::new(HashMap::new())), events }
    }

    pub fn set_status(&self, session_id: Uuid, status: CaptureStatus) {
        self.capture_status.lock().unwrap().insert(session_id, status.clone());
        let _ = self.events.send(CaptureStatusEvent { session_id, status });
    }

    pub fn get_status(&self, session_id: Uuid) -> Option<CaptureStatus> {
        self.capture_status.lock().unwrap().get(&session_id).cloned()
    }
}
