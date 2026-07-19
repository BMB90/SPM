use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The class of configuration/persistence mechanism a `ConfigEntry`
/// represents. Deliberately covers both Windows (registry, COM) and Linux
/// (unit files, udev rules, env files, kernel params) under one model so
/// the UI/analysis layer does not need per-OS branching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigEntryKind {
    RegistryKey,
    RegistryValue,
    ComRegistration,
    WindowsPolicy,
    SystemdUnitFile,
    EnvironmentFile,
    UdevRule,
    ModprobeConfig,
    KernelParameter,
    GenericConfigFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigAccess {
    Read,
    Write,
    Created,
    Deleted,
}

/// A registry key/value (Windows) or configuration file/unit/rule (Linux)
/// that influenced startup behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEntry {
    pub id: Uuid,
    pub session_id: Uuid,

    pub kind: ConfigEntryKind,
    /// e.g. `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Run` or
    /// `/etc/systemd/system/docker.service`.
    pub location: String,
    pub name: Option<String>,
    pub value: Option<String>,

    pub access: ConfigAccess,
    /// PID of the process that read/wrote this entry, when observed live
    /// (vs. discovered via static enumeration at capture time).
    pub pid: Option<u32>,

    /// Startup items whose existence is directly explained by this entry
    /// (e.g. the process paths named by a Run key).
    pub related_startup_items: Vec<String>,
}
