//! Core engine: session management, event ingestion/normalization,
//! dependency-graph reconstruction, and timeline generation. This crate
//! owns no I/O — it consumes `spm_core::Event`s (already produced by
//! collectors) and turns them into the structured views (`Timeline`,
//! `DependencyGraph`, deduplicated entity tables) the API and UI render.

pub mod dependency;
pub mod normalize;
pub mod session_manager;
pub mod timeline_builder;

pub use dependency::DependencyGraphBuilder;
pub use normalize::{EventProcessor, NormalizedSession};
pub use session_manager::SessionManager;
pub use timeline_builder::TimelineBuilder;
