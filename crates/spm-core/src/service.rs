use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::performance::PerformanceMetrics;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    Running,
    Stopped,
    StartPending,
    StopPending,
    Paused,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStartType {
    Boot,
    System,
    Automatic,
    AutomaticDelayedStart,
    Manual,
    Disabled,
    Unknown,
}

/// A Windows Service (SCM) or Linux systemd unit / init.d service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub id: Uuid,
    pub session_id: Uuid,

    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,

    pub binary_path: Option<String>,
    pub config_path: Option<String>,

    pub state: ServiceState,
    pub start_type: ServiceStartType,

    pub owner: Option<String>,
    pub pid: Option<u32>,

    pub depends_on: Vec<String>,
    pub required_by: Vec<String>,

    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,

    pub restart_count: u32,
    pub last_failure: Option<String>,

    pub performance: PerformanceMetrics,
}

impl ServiceInfo {
    pub fn new(session_id: Uuid, name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            name: name.into(),
            display_name: None,
            description: None,
            binary_path: None,
            config_path: None,
            state: ServiceState::Unknown,
            start_type: ServiceStartType::Unknown,
            owner: None,
            pid: None,
            depends_on: Vec::new(),
            required_by: Vec::new(),
            start_time: None,
            end_time: None,
            restart_count: 0,
            last_failure: None,
            performance: PerformanceMetrics::default(),
        }
    }
}
