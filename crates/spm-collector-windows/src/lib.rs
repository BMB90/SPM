//! Windows platform collectors. Each module implements `spm_core::Collector`
//! (or `StreamingCollector`) against one native Windows instrumentation
//! source; nothing here is reachable from `spm-core`, `spm-engine`, or
//! `spm-storage` — only `spm-cli`/`spm-api` compose these into a capture.

#![cfg(windows)]

mod drivers;
mod eventlog;
mod hash;
mod modules;
mod process;
mod registry;
mod scheduled_tasks;
mod services;
mod signature;
mod sysmon;
mod util;

#[cfg(windows)]
mod etw;

pub use drivers::DriverCollector;
pub use eventlog::EventLogCollector;
pub use modules::ModuleCollector;
pub use process::ProcessSnapshotCollector;
pub use registry::StartupRegistryCollector;
pub use scheduled_tasks::ScheduledTaskCollector;
pub use services::ServiceCollector;
pub use sysmon::SysmonCollector;

pub use etw::EtwProcessTraceCollector;

use std::sync::Arc;

use spm_core::{Collector, StreamingCollector};

/// Every snapshot collector, in the order the engine should run them
/// (cheap/fast first). `spm-cli`/`spm-api` use this instead of hand-listing
/// collectors so adding a new one only requires touching this crate.
pub fn all_snapshot_collectors() -> Vec<Arc<dyn Collector>> {
    all_snapshot_collectors_with_options(true)
}

/// Same as [`all_snapshot_collectors`], but lets the caller skip the
/// comparatively expensive per-process hash + Authenticode enrichment
/// pass (useful for fast repeated captures during development).
pub fn all_snapshot_collectors_with_options(enrich_processes: bool) -> Vec<Arc<dyn Collector>> {
    vec![
        Arc::new(ProcessSnapshotCollector { enrich: enrich_processes }),
        Arc::new(StartupRegistryCollector),
        Arc::new(ServiceCollector),
        Arc::new(DriverCollector),
        Arc::new(ModuleCollector),
        Arc::new(ScheduledTaskCollector),
        Arc::new(EventLogCollector),
        Arc::new(SysmonCollector),
    ]
}

/// Every streaming collector (currently just ETW process tracing).
pub fn all_streaming_collectors() -> Vec<Arc<dyn StreamingCollector>> {
    vec![Arc::new(EtwProcessTraceCollector)]
}
