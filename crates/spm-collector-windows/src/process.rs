use std::collections::HashMap;

use async_trait::async_trait;
use spm_core::{
    Collector, CollectorCategory, CollectorContext, Event, EventPayload, ExecutableMetadata, Platform, ProcessInfo,
    ProcessRole, SpmResult,
};
use sysinfo::{System, Users};

use crate::hash::sha256_file;
use crate::signature::check_signature;

/// Snapshots every currently-running process via `sysinfo` (which on
/// Windows is backed by the Toolhelp/NtQuerySystemInformation/PDH APIs),
/// then enriches each with a SHA-256 hash and Authenticode signature
/// status. Hashing/signature-checking is CPU/IO-bound so it runs on the
/// blocking thread pool rather than the async collector task.
pub struct ProcessSnapshotCollector {
    /// When false, skips the (comparatively expensive) hash + signature
    /// enrichment pass — useful for fast repeated captures.
    pub enrich: bool,
}

impl Default for ProcessSnapshotCollector {
    fn default() -> Self {
        Self { enrich: true }
    }
}

#[async_trait]
impl Collector for ProcessSnapshotCollector {
    fn id(&self) -> &'static str {
        "windows.process_snapshot"
    }
    fn platform(&self) -> Platform {
        Platform::Windows
    }
    fn category(&self) -> CollectorCategory {
        CollectorCategory::Process
    }
    fn description(&self) -> &'static str {
        "Enumerates all running processes with full command line, ownership, timing, hash, and signature metadata"
    }

    async fn collect(&self, ctx: &CollectorContext) -> SpmResult<Vec<Event>> {
        let session_id = ctx.session.id;
        let collector_id = self.id();
        let enrich = self.enrich;

        let processes = tokio::task::spawn_blocking(move || snapshot(session_id, enrich))
            .await
            .map_err(|e| spm_core::SpmError::Collector { collector: collector_id.to_string(), message: e.to_string() })?;

        Ok(processes
            .into_iter()
            .map(|p| Event::new(session_id, collector_id, EventPayload::ProcessStarted(Box::new(p))))
            .collect())
    }
}

fn snapshot(session_id: uuid::Uuid, enrich: bool) -> Vec<ProcessInfo> {
    let mut sys = System::new_all();
    sys.refresh_all();
    let users = Users::new_with_refreshed_list();

    let now_unix = chrono::Utc::now().timestamp() as u64;

    sys.processes()
        .values()
        .map(|proc| {
            let pid = proc.pid().as_u32();
            let mut info = ProcessInfo::new(session_id, pid, proc.name().to_string());
            info.ppid = proc.parent().map(|p| p.as_u32());
            info.executable_path = proc.exe().map(|p| p.to_string_lossy().to_string());
            info.working_directory = proc.cwd().map(|p| p.to_string_lossy().to_string());
            info.arguments = proc.cmd().to_vec();
            info.command_line = if info.arguments.is_empty() { None } else { Some(info.arguments.join(" ")) };
            info.environment = parse_environ(proc.environ());

            // sysinfo uses 0 as a "couldn't determine this" sentinel here
            // (common for protected/other-user processes when not
            // elevated) rather than a real epoch-0 timestamp — treat it as
            // unknown, not as 1970-01-01, or every such process corrupts
            // the timeline with a multi-decade bogus offset.
            let start_secs = proc.start_time();
            info.start_time = if start_secs > 0 { chrono::DateTime::from_timestamp(start_secs as i64, 0) } else { None };

            info.user = proc
                .user_id()
                .and_then(|uid| users.iter().find(|u| u.id() == uid))
                .map(|u| u.name().to_string());

            info.performance.cpu_percent_peak = Some(proc.cpu_usage());
            info.performance.cpu_percent_avg = Some(proc.cpu_usage());
            info.performance.memory_bytes_current = Some(proc.memory());
            info.performance.memory_bytes_peak = Some(proc.memory());
            info.performance.disk_read_bytes = Some(proc.disk_usage().total_read_bytes);
            info.performance.disk_write_bytes = Some(proc.disk_usage().total_written_bytes);
            let run_time = proc.run_time();
            if run_time > 0 && now_unix >= start_secs {
                info.performance.cpu_time_ms = None; // sysinfo doesn't expose total CPU time directly on Windows
            }

            info.role = infer_role(pid, info.ppid);
            info.metadata = ExecutableMetadata::default();

            if enrich {
                if let Some(path) = &info.executable_path {
                    info.sha256 = sha256_file(path);
                    info.signature_status = check_signature(path);
                }
            }

            info
        })
        .collect()
}

fn parse_environ(entries: &[String]) -> HashMap<String, String> {
    entries
        .iter()
        .filter_map(|s| s.split_once('=').map(|(k, v)| (k.to_string(), v.to_string())))
        .collect()
}

fn infer_role(pid: u32, ppid: Option<u32>) -> ProcessRole {
    match (pid, ppid) {
        (0, _) => ProcessRole::KernelProcess,
        (4, _) => ProcessRole::KernelProcess,
        (p, _) if p <= 8 => ProcessRole::System,
        _ => ProcessRole::Unknown,
    }
}
