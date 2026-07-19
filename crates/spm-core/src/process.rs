use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::performance::PerformanceMetrics;
use crate::security::SecurityInfo;
use crate::startup_source::StartupSource;

/// What role a process plays in the startup chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessRole {
    KernelProcess,
    System,
    Service,
    Daemon,
    ScheduledTask,
    LoginItem,
    UserApplication,
    Unknown,
}

/// Code-signing / package verification status of an executable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SignatureStatus {
    Signed,
    SignedUntrusted,
    Unsigned,
    #[default]
    Unknown,
}

/// Version / descriptive metadata pulled from the executable itself
/// (PE version resource on Windows, package metadata on Linux).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutableMetadata {
    pub version: Option<String>,
    pub description: Option<String>,
    pub company: Option<String>,
    pub product_name: Option<String>,
    pub compile_timestamp: Option<DateTime<Utc>>,
    /// Owning package, e.g. `apt`/`dpkg`/`rpm` package name on Linux.
    pub package: Option<String>,
}

/// Full forensic record for one process observed during the capture
/// session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub id: Uuid,
    pub session_id: Uuid,

    pub pid: u32,
    pub ppid: Option<u32>,

    pub executable_name: String,
    pub executable_path: Option<String>,
    pub working_directory: Option<String>,
    pub command_line: Option<String>,
    pub arguments: Vec<String>,
    pub environment: HashMap<String, String>,

    pub start_time: Option<DateTime<Utc>>,
    pub exit_time: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,

    pub user: Option<String>,
    pub group: Option<String>,

    pub thread_count: Option<u32>,
    pub handle_count: Option<u32>,

    pub sha256: Option<String>,
    pub signature_status: SignatureStatus,
    pub signer: Option<String>,
    pub metadata: ExecutableMetadata,

    pub role: ProcessRole,
    pub owning_service: Option<String>,

    pub startup_source: Option<StartupSource>,
    pub security: SecurityInfo,
    pub performance: PerformanceMetrics,
}

impl ProcessInfo {
    pub fn new(session_id: Uuid, pid: u32, executable_name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            pid,
            ppid: None,
            executable_name: executable_name.into(),
            executable_path: None,
            working_directory: None,
            command_line: None,
            arguments: Vec::new(),
            environment: HashMap::new(),
            start_time: None,
            exit_time: None,
            exit_code: None,
            user: None,
            group: None,
            thread_count: None,
            handle_count: None,
            sha256: None,
            signature_status: SignatureStatus::default(),
            signer: None,
            metadata: ExecutableMetadata::default(),
            role: ProcessRole::Unknown,
            owning_service: None,
            startup_source: None,
            security: SecurityInfo::default(),
            performance: PerformanceMetrics::default(),
        }
    }

    /// Wall-clock lifetime of the process, if it has already exited.
    pub fn lifetime(&self) -> Option<chrono::Duration> {
        match (self.start_time, self.exit_time) {
            (Some(start), Some(end)) => Some(end - start),
            _ => None,
        }
    }
}
