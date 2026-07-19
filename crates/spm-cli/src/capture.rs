use std::time::Duration;

use anyhow::Result;
use spm_orchestrator::CaptureOptions;
use spm_storage::Storage;
use uuid::Uuid;

use crate::platform_collectors;

/// Thin adapter: supplies this platform's collectors to
/// `spm_orchestrator::run_capture`, plus the CLI-only `--no-enrich` flag
/// (the orchestrator itself doesn't know about per-collector options).
pub async fn run_capture(storage: &Storage, capture_window: Duration, enrich: bool, notes: Option<String>) -> Result<Uuid> {
    let snapshot = platform_collectors::snapshot_collectors(enrich);
    let streaming = platform_collectors::streaming_collectors();
    spm_orchestrator::run_capture(storage, snapshot, streaming, CaptureOptions { capture_window, notes }).await
}
