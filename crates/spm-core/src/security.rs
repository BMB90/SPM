use serde::{Deserialize, Serialize};

/// Security-relevant context captured for a process or startup item.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityInfo {
    /// Windows integrity level (e.g. "System", "High", "Medium", "Low") or
    /// a Linux-equivalent summary (e.g. "root", "unprivileged").
    pub integrity_level: Option<String>,
    /// Linux capabilities (e.g. `CAP_NET_ADMIN`) or Windows privileges
    /// (e.g. `SeDebugPrivilege`) held by the process token.
    pub privileges: Vec<String>,
    pub group_memberships: Vec<String>,
    pub is_elevated: Option<bool>,
    /// Findings raised by `spm-analysis` heuristics (empty until analyzed).
    pub findings: Vec<SecurityFinding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub severity: FindingSeverity,
    pub code: String,
    pub message: String,
}
