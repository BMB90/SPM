use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileOperation {
    Read,
    Write,
    Create,
    Delete,
    Rename,
    PermissionChange,
    OwnerChange,
}

/// A single startup-relevant file-system operation, typically sourced from
/// fanotify/audit on Linux or ETW file-IO/Sysmon events on Windows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileActivity {
    pub id: Uuid,
    pub session_id: Uuid,

    pub operation: FileOperation,
    pub path: String,
    pub new_path: Option<String>,
    pub owner: Option<String>,

    pub pid: u32,
    pub process_executable: Option<String>,

    pub timestamp: DateTime<Utc>,
}
