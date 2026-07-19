//! Isolates the only place in `spm-cli` that knows which collector crate
//! to use for the current OS. Adding a third platform means adding one
//! more `#[cfg(...)]` arm here — nothing else in this crate changes.

use std::sync::Arc;

use spm_core::{Collector, StreamingCollector};

#[cfg(windows)]
pub fn snapshot_collectors(enrich: bool) -> Vec<Arc<dyn Collector>> {
    spm_collector_windows::all_snapshot_collectors_with_options(enrich)
}

#[cfg(windows)]
pub fn streaming_collectors() -> Vec<Arc<dyn StreamingCollector>> {
    spm_collector_windows::all_streaming_collectors()
}

#[cfg(all(target_os = "linux", not(windows)))]
pub fn snapshot_collectors(_enrich: bool) -> Vec<Arc<dyn Collector>> {
    spm_collector_linux::all_snapshot_collectors()
}

#[cfg(all(target_os = "linux", not(windows)))]
pub fn streaming_collectors() -> Vec<Arc<dyn StreamingCollector>> {
    spm_collector_linux::all_streaming_collectors()
}
