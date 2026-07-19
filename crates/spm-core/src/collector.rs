use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, watch};

use crate::error::SpmResult;
use crate::event::Event;
use crate::platform::Platform;
use crate::session::BootSession;

/// Broad category a collector belongs to. Used for filtering, UI grouping,
/// and letting `spm-engine` decide default collection order/timeouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectorCategory {
    Process,
    Service,
    Driver,
    Module,
    FileActivity,
    Network,
    Configuration,
    BootStage,
}

/// Shared context handed to every collector invocation.
#[derive(Clone)]
pub struct CollectorContext {
    pub session: BootSession,
    /// How long a streaming collector (e.g. ETW, eBPF) should run before
    /// the engine considers the capture window closed. Snapshot collectors
    /// ignore this.
    pub capture_window: Duration,
    /// Cooperative cancellation signal; collectors doing bounded streaming
    /// should select on this alongside their event source.
    pub cancel: watch::Receiver<bool>,
}

impl CollectorContext {
    pub fn is_cancelled(&self) -> bool {
        *self.cancel.borrow()
    }
}

/// A collector that produces a finite batch of events for the current
/// point in time (registry enumeration, WMI queries, service/driver
/// enumeration, process snapshots, ...).
#[async_trait]
pub trait Collector: Send + Sync {
    /// Stable machine-readable identifier, e.g. `"windows.registry_run_keys"`.
    fn id(&self) -> &'static str;
    fn platform(&self) -> Platform;
    fn category(&self) -> CollectorCategory;
    fn description(&self) -> &'static str;

    /// Whether this collector can run in the current environment (e.g.
    /// sufficient privilege, required OS component present). The engine
    /// skips unavailable collectors and records why.
    fn is_available(&self) -> SpmResult<()> {
        Ok(())
    }

    async fn collect(&self, ctx: &CollectorContext) -> SpmResult<Vec<Event>>;
}

/// A collector that produces a continuous stream of events for the
/// duration of `CollectorContext::capture_window` (ETW/eBPF/audit
/// consumers). The engine drains `tx` until the collector returns or the
/// capture window elapses.
#[async_trait]
pub trait StreamingCollector: Send + Sync {
    fn id(&self) -> &'static str;
    fn platform(&self) -> Platform;
    fn category(&self) -> CollectorCategory;
    fn description(&self) -> &'static str;

    fn is_available(&self) -> SpmResult<()> {
        Ok(())
    }

    async fn stream(&self, ctx: &CollectorContext, tx: mpsc::Sender<Event>) -> SpmResult<()>;
}
