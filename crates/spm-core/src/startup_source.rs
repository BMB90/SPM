use serde::{Deserialize, Serialize};

/// The mechanism that triggered a process, service, or driver to start.
///
/// Every variant is expected to carry enough information in
/// `StartupSource::evidence` to justify the classification to a human
/// analyst (e.g. the exact registry key, unit file path, or parent PID).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StartupSourceKind {
    SystemdService { unit: String },
    SystemdTimer { unit: String },
    Cron { entry: String, schedule: String },
    InitScript { path: String },
    AutostartDesktopEntry { path: String },
    ShellStartupScript { path: String },
    UserLogin,
    RegistryRunKey { hive: String, key: String, value: String },
    StartupFolder { path: String },
    ScheduledTask { task_path: String },
    WindowsService { service_name: String },
    KernelLaunch,
    DriverInit { driver: String },
    ParentProcess { parent_pid: u32, parent_executable: Option<String> },
    ComActivation { clsid: String },
    ShellExtension { clsid: String },
    Udev { rule: String },
    Other { description: String },
    Unknown,
}

/// A startup source classification with supporting evidence, as required
/// by the spec's "Startup Source Detection" requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupSource {
    pub kind: StartupSourceKind,
    /// Human-readable evidence trail, e.g. the literal registry value data,
    /// the unit file contents fragment, or the audit log record id that
    /// justified this classification.
    pub evidence: Vec<String>,
    /// 0.0-1.0 confidence in this classification. Direct observation (e.g.
    /// audit subsystem correlated exec to a specific unit) should be 1.0;
    /// heuristic inference (e.g. "started shortly after explorer.exe, in
    /// the Startup folder scan") should be lower.
    pub confidence: f32,
}

impl StartupSource {
    pub fn new(kind: StartupSourceKind) -> Self {
        Self { kind, evidence: Vec::new(), confidence: 1.0 }
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence.push(evidence.into());
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}
