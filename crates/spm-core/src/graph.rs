use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    ParentChild,
    ServiceDependency,
    DriverDependency,
    LibraryDependency,
    NetworkDependency,
    FilesystemDependency,
    PackageDependency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Kernel,
    Process,
    Service,
    Driver,
    Module,
}

/// One node in the dependency graph (a process, service, driver, or
/// module).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: NodeKind,
    pub label: String,
    /// Free-form key/value pairs for UI tooltips (pid, path, state, ...).
    pub attributes: std::collections::HashMap<String, String>,
}

/// One directed edge in the dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub id: Uuid,
    pub session_id: Uuid,
    pub from: String,
    pub to: String,
    pub kind: DependencyKind,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<DependencyEdge>,
}
