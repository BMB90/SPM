//! Cross-platform domain model, common event model, and the `Collector`
//! trait that every OS-specific collector implements. This crate has no
//! platform-specific dependencies and no I/O of its own — it exists so
//! `spm-engine`, `spm-storage`, `spm-analysis`, `spm-api`, and both
//! `spm-collector-*` crates can share one vocabulary without any of them
//! depending on the others.

pub mod collector;
pub mod config_entry;
pub mod driver;
pub mod error;
pub mod event;
pub mod file_activity;
pub mod graph;
pub mod module;
pub mod network;
pub mod performance;
pub mod platform;
pub mod process;
pub mod security;
pub mod service;
pub mod session;
pub mod startup_source;
pub mod timeline;

pub use collector::{Collector, CollectorCategory, CollectorContext, StreamingCollector};
pub use config_entry::{ConfigAccess, ConfigEntry, ConfigEntryKind};
pub use driver::{DriverInfo, DriverStatus};
pub use error::{SpmError, SpmResult};
pub use event::{Event, EventPayload};
pub use file_activity::{FileActivity, FileOperation};
pub use graph::{DependencyEdge, DependencyGraph, DependencyKind, GraphNode, NodeKind};
pub use module::{ModuleInfo, ModuleKind};
pub use network::{NetworkActivity, NetworkProtocol};
pub use performance::PerformanceMetrics;
pub use platform::Platform;
pub use process::{ExecutableMetadata, ProcessInfo, ProcessRole, SignatureStatus};
pub use security::{FindingSeverity, SecurityFinding, SecurityInfo};
pub use service::{ServiceInfo, ServiceStartType, ServiceState};
pub use session::{BootSession, BootStage};
pub use startup_source::{StartupSource, StartupSourceKind};
pub use timeline::TimelineEntry;
