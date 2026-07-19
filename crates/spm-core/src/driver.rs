use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::process::SignatureStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriverStatus {
    Running,
    Stopped,
    Failed,
    Unknown,
}

/// A kernel driver (Windows `.sys`) or Linux kernel module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverInfo {
    pub id: Uuid,
    pub session_id: Uuid,

    pub name: String,
    pub path: Option<String>,

    pub load_order: Option<u32>,
    pub load_time: Option<DateTime<Utc>>,
    pub unload_time: Option<DateTime<Utc>>,

    pub version: Option<String>,
    pub vendor: Option<String>,
    pub signature_status: SignatureStatus,

    pub depends_on: Vec<String>,
    pub status: DriverStatus,
    pub failure_reason: Option<String>,
}

impl DriverInfo {
    pub fn new(session_id: Uuid, name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            name: name.into(),
            path: None,
            load_order: None,
            load_time: None,
            unload_time: None,
            version: None,
            vendor: None,
            signature_status: SignatureStatus::default(),
            depends_on: Vec::new(),
            status: DriverStatus::Unknown,
            failure_reason: None,
        }
    }
}
