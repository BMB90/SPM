use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config_entry::ConfigEntry;
use crate::driver::DriverInfo;
use crate::file_activity::FileActivity;
use crate::module::ModuleInfo;
use crate::network::NetworkActivity;
use crate::process::ProcessInfo;
use crate::service::ServiceInfo;
use crate::session::BootStage;

/// The common event model every platform collector normalizes into. This
/// is the single seam between OS-specific instrumentation (ETW, eBPF,
/// audit, WMI, ...) and the platform-agnostic core engine / storage /
/// analysis layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum EventPayload {
    ProcessStarted(Box<ProcessInfo>),
    ProcessUpdated(Box<ProcessInfo>),
    ProcessExited { pid: u32, exit_code: Option<i32>, exit_time: DateTime<Utc> },

    ServiceObserved(Box<ServiceInfo>),
    ServiceStateChanged { name: String, state: crate::service::ServiceState, timestamp: DateTime<Utc> },

    DriverObserved(Box<DriverInfo>),
    ModuleLoaded(Box<ModuleInfo>),

    FileActivityObserved(Box<FileActivity>),
    NetworkActivityObserved(Box<NetworkActivity>),
    ConfigEntryObserved(Box<ConfigEntry>),

    BootStageReached { stage: BootStage, timestamp: DateTime<Utc>, detail: Option<String> },

    /// Escape hatch for collector-specific data that doesn't yet have a
    /// first-class model (kept structured so nothing is silently dropped).
    Raw { source: String, kind: String, payload: serde_json::Value },
}

/// Envelope wrapping every `EventPayload` with provenance metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub session_id: Uuid,
    pub collector_id: String,
    pub observed_at: DateTime<Utc>,
    pub payload: EventPayload,
}

impl Event {
    pub fn new(session_id: Uuid, collector_id: impl Into<String>, payload: EventPayload) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            collector_id: collector_id.into(),
            observed_at: Utc::now(),
            payload,
        }
    }
}
