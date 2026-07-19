use serde::{Deserialize, Serialize};

/// Point-in-time or aggregated resource-usage metrics for a process,
/// service, or driver.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub cpu_time_ms: Option<u64>,
    pub cpu_percent_avg: Option<f32>,
    pub cpu_percent_peak: Option<f32>,

    pub memory_bytes_current: Option<u64>,
    pub memory_bytes_peak: Option<u64>,

    pub disk_read_bytes: Option<u64>,
    pub disk_write_bytes: Option<u64>,

    pub network_rx_bytes: Option<u64>,
    pub network_tx_bytes: Option<u64>,

    pub thread_count_peak: Option<u32>,
    pub context_switches: Option<u64>,

    /// Time from process/service/driver start until it reported ready
    /// (or, absent an explicit readiness signal, until its main thread's
    /// first idle wait) in milliseconds.
    pub init_duration_ms: Option<u64>,
}
