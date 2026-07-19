//! Same pattern as `spm-cli`'s `platform_collectors` module — isolates
//! the one place this crate knows which collector crate to use.

use std::sync::Arc;

use spm_core::{Collector, StreamingCollector};

#[cfg(windows)]
pub fn snapshot_collectors() -> Vec<Arc<dyn Collector>> {
    spm_collector_windows::all_snapshot_collectors()
}

#[cfg(windows)]
pub fn streaming_collectors() -> Vec<Arc<dyn StreamingCollector>> {
    spm_collector_windows::all_streaming_collectors()
}

#[cfg(all(target_os = "linux", not(windows)))]
pub fn snapshot_collectors() -> Vec<Arc<dyn Collector>> {
    spm_collector_linux::all_snapshot_collectors()
}

#[cfg(all(target_os = "linux", not(windows)))]
pub fn streaming_collectors() -> Vec<Arc<dyn StreamingCollector>> {
    spm_collector_linux::all_streaming_collectors()
}
