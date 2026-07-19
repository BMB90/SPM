//! Startup-source detection, suspicious-activity heuristics, and
//! historical (session-to-session) comparison. Dependency-graph and
//! timeline reconstruction live in `spm-engine` since they operate purely
//! on the in-memory `NormalizedSession` during a single capture; this
//! crate handles the two analyses that need cross-referencing config
//! entries against processes (`startup_source`) or reading back persisted
//! sessions from storage (`comparison`).

pub mod comparison;
pub mod reporting;
pub mod security;
pub mod startup_source;

pub use comparison::{HistoricalComparator, PathChange, SessionComparison, SetDelta};
pub use reporting::{ReportGenerator, SessionReport};
pub use security::SecurityAnalyzer;
pub use startup_source::StartupSourceDetector;
