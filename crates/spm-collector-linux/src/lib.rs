//! Linux platform collectors.
//!
//! **Status: interface-only stubs.** Every collector below implements the
//! real `spm_core::Collector`/`StreamingCollector` traits and returns a
//! well-formed (empty) result, so the engine, storage, analysis, and API
//! layers all work unmodified on Linux today — but none of them read real
//! system state yet. Implementing each is a self-contained follow-up (see
//! `docs/collector-architecture.md`); this crate exists so that work has
//! an obvious, tested slot rather than requiring core-engine changes.
//!
//! Planned real implementations, by collector:
//! - `ProcessCollector` — `/proc/<pid>/{stat,status,cmdline,environ,exe,cwd,fd}`
//! - `SystemdCollector` — systemd D-Bus API (`org.freedesktop.systemd1`) for units + `systemd-analyze` timing
//! - `KernelModuleCollector` — `/proc/modules` + `/sys/module/*`
//! - `AuditCollector` — Linux audit subsystem (`auditd` netlink socket) for exec/file events
//! - `EbpfProcessTraceCollector` — eBPF tracepoints (`sched_process_exec`/`sched_process_exit`) via `aya` or `libbpf-rs`
//! - `UdevCollector` — `udevadm`/netlink for device discovery events
//! - `CronCollector` — `/etc/cron*`, `/var/spool/cron`, user crontabs
//! - `JournalCollector` — `systemd-journal` (via `sd-journal` bindings) for boot-time log correlation

#![cfg(target_os = "linux")]

use std::sync::Arc;

use async_trait::async_trait;
use spm_core::{Collector, CollectorCategory, CollectorContext, Event, Platform, SpmError, SpmResult, StreamingCollector};

macro_rules! stub_snapshot_collector {
    ($name:ident, $id:literal, $category:expr, $description:literal) => {
        pub struct $name;

        #[async_trait]
        impl Collector for $name {
            fn id(&self) -> &'static str {
                $id
            }
            fn platform(&self) -> Platform {
                Platform::Linux
            }
            fn category(&self) -> CollectorCategory {
                $category
            }
            fn description(&self) -> &'static str {
                $description
            }
            fn is_available(&self) -> SpmResult<()> {
                Err(SpmError::Unavailable {
                    collector: self.id().to_string(),
                    reason: "not yet implemented — interface-only stub, see spm-collector-linux crate docs".to_string(),
                })
            }
            async fn collect(&self, _ctx: &CollectorContext) -> SpmResult<Vec<Event>> {
                Ok(Vec::new())
            }
        }
    };
}

stub_snapshot_collector!(ProcessCollector, "linux.process_snapshot", CollectorCategory::Process, "procfs-based process enumeration (planned)");
stub_snapshot_collector!(SystemdCollector, "linux.systemd", CollectorCategory::Service, "systemd unit inventory via D-Bus (planned)");
stub_snapshot_collector!(KernelModuleCollector, "linux.kernel_modules", CollectorCategory::Driver, "/proc/modules + /sys/module enumeration (planned)");
stub_snapshot_collector!(UdevCollector, "linux.udev", CollectorCategory::Configuration, "udev device-discovery events (planned)");
stub_snapshot_collector!(CronCollector, "linux.cron", CollectorCategory::Configuration, "cron/crontab startup-source enumeration (planned)");
stub_snapshot_collector!(JournalCollector, "linux.journal", CollectorCategory::BootStage, "systemd-journal boot correlation (planned)");
stub_snapshot_collector!(AuditCollector, "linux.audit", CollectorCategory::FileActivity, "Linux audit subsystem exec/file events (planned)");

pub struct EbpfProcessTraceCollector;

#[async_trait]
impl StreamingCollector for EbpfProcessTraceCollector {
    fn id(&self) -> &'static str {
        "linux.ebpf_process_trace"
    }
    fn platform(&self) -> Platform {
        Platform::Linux
    }
    fn category(&self) -> CollectorCategory {
        CollectorCategory::Process
    }
    fn description(&self) -> &'static str {
        "Real-time process exec/exit tracing via eBPF tracepoints (planned)"
    }
    fn is_available(&self) -> SpmResult<()> {
        Err(SpmError::Unavailable {
            collector: self.id().to_string(),
            reason: "not yet implemented — interface-only stub, see spm-collector-linux crate docs".to_string(),
        })
    }
    async fn stream(&self, _ctx: &CollectorContext, _tx: tokio::sync::mpsc::Sender<Event>) -> SpmResult<()> {
        Ok(())
    }
}

/// Every snapshot collector defined for Linux. All currently report
/// `is_available() == Err(..)`, so `spm-engine`'s capture orchestrator
/// skips them and logs why — the capture still completes cleanly.
pub fn all_snapshot_collectors() -> Vec<Arc<dyn Collector>> {
    vec![
        Arc::new(ProcessCollector),
        Arc::new(SystemdCollector),
        Arc::new(KernelModuleCollector),
        Arc::new(UdevCollector),
        Arc::new(CronCollector),
        Arc::new(JournalCollector),
        Arc::new(AuditCollector),
    ]
}

pub fn all_streaming_collectors() -> Vec<Arc<dyn StreamingCollector>> {
    vec![Arc::new(EbpfProcessTraceCollector)]
}
