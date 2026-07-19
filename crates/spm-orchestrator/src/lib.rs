//! Shared capture orchestration: collect → normalize → detect startup
//! sources → run security heuristics → build the dependency graph +
//! timeline → persist. This crate takes collectors as parameters (it
//! never constructs them itself), so it stays platform-agnostic — both
//! `spm-cli` and `spm-api` supply the platform-appropriate collector
//! lists from their own thin `platform_collectors` module and call the
//! same functions here, so the two entry points can never drift.
//!
//! [`run_capture`] is the simple synchronous-shaped entry point (used by
//! the CLI, which just wants a session id when it's done). [`begin_session`]
//! + [`run_capture_for_session`] split that in two so a caller — the API
//! server — can persist the session and hand a session id back to its
//! client *before* the (multi-second) capture finishes, then run the rest
//! in the background.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use spm_analysis::{SecurityAnalyzer, StartupSourceDetector};
use spm_core::{BootSession, Collector, StreamingCollector};
use spm_engine::{DependencyGraphBuilder, SessionManager, TimelineBuilder};
use spm_storage::Storage;
use sysinfo::System;
use uuid::Uuid;

pub struct CaptureOptions {
    pub capture_window: Duration,
    pub notes: Option<String>,
}

impl Default for CaptureOptions {
    fn default() -> Self {
        Self { capture_window: Duration::from_secs(5), notes: None }
    }
}

/// Creates and persists a new `BootSession` row for the current host.
/// Returns immediately — no collection has happened yet.
pub fn begin_session(storage: &Storage, notes: Option<String>) -> Result<BootSession> {
    let manager = SessionManager::new(spm_core::Platform::CURRENT);
    let hostname = System::host_name().unwrap_or_else(|| "unknown-host".to_string());
    let os_version =
        System::long_os_version().unwrap_or_else(|| format!("{} {}", std::env::consts::OS, std::env::consts::ARCH));

    let mut session = manager.new_session(hostname, os_version);
    session.notes = notes;
    // `System::boot_time()` returns the system's actual last-boot time
    // (seconds since Unix epoch), independent of when this capture
    // process started — without this, timeline offsets would be computed
    // against `capture_started_at` and every already-running process
    // would show as having started an enormous negative offset ago.
    let boot_time_secs = System::boot_time();
    if boot_time_secs > 0 {
        session.boot_time = chrono::DateTime::from_timestamp(boot_time_secs as i64, 0);
    }
    storage.create_session(&session)?;
    Ok(session)
}

/// Runs collection + normalization + analysis + persistence for an
/// already-created session. Does not create or complete-check the
/// session row beyond marking it complete at the end.
pub async fn run_capture_for_session(
    storage: &Storage,
    session: &BootSession,
    snapshot_collectors: Vec<Arc<dyn Collector>>,
    streaming_collectors: Vec<Arc<dyn StreamingCollector>>,
    capture_window: Duration,
) -> Result<()> {
    let manager = SessionManager::new(session.platform);

    tracing::info!(session_id = %session.id, "starting capture");

    let mut normalized = manager.capture(session, snapshot_collectors, streaming_collectors, capture_window).await;

    StartupSourceDetector::detect(&mut normalized);
    SecurityAnalyzer::analyze(&mut normalized);

    let graph = DependencyGraphBuilder::build(session.id, &normalized);
    let critical_path = DependencyGraphBuilder::critical_path(&graph, &normalized);
    let timeline = TimelineBuilder::build(session, &normalized, &critical_path);

    let processes: Vec<_> = normalized.processes.into_values().collect();
    let services: Vec<_> = normalized.services.into_values().collect();
    let drivers: Vec<_> = normalized.drivers.into_values().collect();

    if !processes.is_empty() {
        storage.save_processes(&processes)?;
    }
    if !services.is_empty() {
        storage.save_services(&services)?;
    }
    if !drivers.is_empty() {
        storage.save_drivers(&drivers)?;
    }
    if !normalized.modules.is_empty() {
        storage.save_modules(&normalized.modules)?;
    }
    if !normalized.file_activity.is_empty() {
        storage.save_file_activity(&normalized.file_activity)?;
    }
    if !normalized.network_activity.is_empty() {
        storage.save_network_activity(&normalized.network_activity)?;
    }
    if !normalized.config_entries.is_empty() {
        storage.save_config_entries(&normalized.config_entries)?;
    }
    if !timeline.is_empty() {
        storage.save_timeline(&timeline)?;
    }
    storage.save_graph(session.id, &graph)?;
    storage.complete_session(session.id, chrono::Utc::now())?;

    tracing::info!(
        session_id = %session.id,
        processes = processes.len(),
        services = services.len(),
        drivers = drivers.len(),
        timeline_entries = timeline.len(),
        "capture complete"
    );

    Ok(())
}

/// Convenience wrapper: creates the session and runs the full capture,
/// returning only once everything is persisted.
pub async fn run_capture(
    storage: &Storage,
    snapshot_collectors: Vec<Arc<dyn Collector>>,
    streaming_collectors: Vec<Arc<dyn StreamingCollector>>,
    options: CaptureOptions,
) -> Result<Uuid> {
    let session = begin_session(storage, options.notes)?;
    run_capture_for_session(storage, &session, snapshot_collectors, streaming_collectors, options.capture_window).await?;
    Ok(session.id)
}
