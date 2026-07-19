use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::platform::Platform;

/// A single captured boot/startup analysis session.
///
/// A session bounds every event, process, service, driver, file, and network
/// record collected during one capture run. Sessions are the unit of
/// historical comparison (see `spm-analysis`'s regression/diff engine).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootSession {
    pub id: Uuid,
    pub hostname: String,
    pub platform: Platform,
    pub os_version: String,
    /// Best-effort kernel/system boot timestamp (e.g. from `/proc/stat
    /// btime` on Linux or `GetTickCount64` derived wall time on Windows).
    /// `None` when the collector could not determine it (e.g. a live
    /// snapshot capture rather than a true cold-boot capture).
    pub boot_time: Option<DateTime<Utc>>,
    /// When this capture session was started (may be well after `boot_time`
    /// if SPM was launched after the machine already reached idle desktop).
    pub capture_started_at: DateTime<Utc>,
    pub capture_completed_at: Option<DateTime<Utc>>,
    pub spm_version: String,
    pub notes: Option<String>,
}

impl BootSession {
    pub fn new(hostname: impl Into<String>, platform: Platform, os_version: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            hostname: hostname.into(),
            platform,
            os_version: os_version.into(),
            boot_time: None,
            capture_started_at: Utc::now(),
            capture_completed_at: None,
            spm_version: env!("CARGO_PKG_VERSION").to_string(),
            notes: None,
        }
    }
}

/// Canonical stages of the startup chain, used to bucket timeline entries
/// regardless of which OS produced them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BootStage {
    Firmware,
    Bootloader,
    Kernel,
    DriverInit,
    FilesystemMount,
    DeviceDiscovery,
    ServiceStartup,
    NetworkInit,
    LoginManager,
    UserLogin,
    DesktopInit,
    StartupApplications,
    ScheduledTasks,
    BackgroundDaemons,
    DesktopReady,
    Idle,
    Unknown,
}

impl std::fmt::Display for BootStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BootStage::Firmware => "firmware",
            BootStage::Bootloader => "bootloader",
            BootStage::Kernel => "kernel",
            BootStage::DriverInit => "driver_init",
            BootStage::FilesystemMount => "filesystem_mount",
            BootStage::DeviceDiscovery => "device_discovery",
            BootStage::ServiceStartup => "service_startup",
            BootStage::NetworkInit => "network_init",
            BootStage::LoginManager => "login_manager",
            BootStage::UserLogin => "user_login",
            BootStage::DesktopInit => "desktop_init",
            BootStage::StartupApplications => "startup_applications",
            BootStage::ScheduledTasks => "scheduled_tasks",
            BootStage::BackgroundDaemons => "background_daemons",
            BootStage::DesktopReady => "desktop_ready",
            BootStage::Idle => "idle",
            BootStage::Unknown => "unknown",
        };
        write!(f, "{s}")
    }
}
