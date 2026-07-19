use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::process::SignatureStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    Dll,
    SharedLibrary,
    KernelModule,
    Plugin,
    DynamicModule,
}

/// A DLL / shared library / kernel module / plugin loaded by a process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub id: Uuid,
    pub session_id: Uuid,

    pub kind: ModuleKind,
    pub name: String,
    pub path: Option<String>,
    pub version: Option<String>,
    pub signature_status: SignatureStatus,

    pub load_time: Option<DateTime<Utc>>,
    /// PID of the process that loaded this module.
    pub parent_pid: u32,
}
