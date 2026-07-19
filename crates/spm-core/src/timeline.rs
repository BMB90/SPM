use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::session::BootStage;

/// One entry on the reconstructed boot timeline (e.g. "Docker started at
/// 2.80s, took 320ms").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub id: Uuid,
    pub session_id: Uuid,

    pub stage: BootStage,
    pub label: String,

    pub timestamp: DateTime<Utc>,
    /// Seconds since `BootSession::boot_time` (or since capture start when
    /// boot_time is unknown). This is the value the UI plots on the x-axis.
    pub offset_seconds: f64,
    pub duration_ms: Option<u64>,

    /// Correlates back to the originating domain object (process id,
    /// service name, driver name, ...) for drill-down in the UI.
    pub subject_kind: String,
    pub subject_id: String,

    /// True when this entry ran concurrently with siblings at the same
    /// stage (used for the "parallel execution visualization").
    pub parallel_group: Option<String>,
    /// True when this entry sits on the critical path (longest dependency
    /// chain) computed by `spm-analysis`.
    pub on_critical_path: bool,
}
