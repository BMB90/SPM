use std::collections::{HashMap, HashSet};

use spm_core::{DependencyEdge, DependencyGraph, DependencyKind, GraphNode, NodeKind};
use uuid::Uuid;

use crate::normalize::NormalizedSession;

const KERNEL_NODE_ID: &str = "kernel";

/// Reconstructs the parent-child, service, driver, and module dependency
/// graph from a `NormalizedSession`, and computes the critical path (the
/// longest chain of process lifetimes from kernel to the last-finishing
/// leaf) used by the timeline UI to highlight what's actually gating boot
/// completion.
pub struct DependencyGraphBuilder;

impl DependencyGraphBuilder {
    pub fn build(session_id: Uuid, normalized: &NormalizedSession) -> DependencyGraph {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        nodes.push(GraphNode {
            id: KERNEL_NODE_ID.to_string(),
            kind: NodeKind::Kernel,
            label: "Kernel".to_string(),
            attributes: HashMap::new(),
        });

        for process in normalized.processes.values() {
            let node_id = process_node_id(process.pid);
            let mut attributes = HashMap::new();
            attributes.insert("pid".to_string(), process.pid.to_string());
            if let Some(path) = &process.executable_path {
                attributes.insert("path".to_string(), path.clone());
            }
            attributes.insert("role".to_string(), format!("{:?}", process.role));

            nodes.push(GraphNode {
                id: node_id.clone(),
                kind: NodeKind::Process,
                label: process.executable_name.clone(),
                attributes,
            });

            let parent_id = match process.ppid {
                Some(ppid) if normalized.processes.contains_key(&ppid) => process_node_id(ppid),
                _ => KERNEL_NODE_ID.to_string(),
            };

            edges.push(DependencyEdge {
                id: Uuid::new_v4(),
                session_id,
                from: parent_id,
                to: node_id.clone(),
                kind: DependencyKind::ParentChild,
                evidence: process.ppid.map(|p| format!("PPID {p}")),
            });

            if let Some(service_name) = &process.owning_service {
                if normalized.services.contains_key(service_name) {
                    edges.push(DependencyEdge {
                        id: Uuid::new_v4(),
                        session_id,
                        from: service_node_id(service_name),
                        to: node_id.clone(),
                        kind: DependencyKind::ServiceDependency,
                        evidence: Some(format!("service '{service_name}' owns pid {}", process.pid)),
                    });
                }
            }
        }

        for service in normalized.services.values() {
            let node_id = service_node_id(&service.name);
            let mut attributes = HashMap::new();
            attributes.insert("state".to_string(), format!("{:?}", service.state));
            if let Some(pid) = service.pid {
                attributes.insert("pid".to_string(), pid.to_string());
            }
            nodes.push(GraphNode {
                id: node_id.clone(),
                kind: NodeKind::Service,
                label: service.display_name.clone().unwrap_or_else(|| service.name.clone()),
                attributes,
            });

            for dep in &service.depends_on {
                edges.push(DependencyEdge {
                    id: Uuid::new_v4(),
                    session_id,
                    from: service_node_id(dep),
                    to: node_id.clone(),
                    kind: DependencyKind::ServiceDependency,
                    evidence: Some(format!("'{}' declares dependency on '{}'", service.name, dep)),
                });
            }
        }

        for driver in normalized.drivers.values() {
            let node_id = driver_node_id(&driver.name);
            nodes.push(GraphNode {
                id: node_id.clone(),
                kind: NodeKind::Driver,
                label: driver.name.clone(),
                attributes: HashMap::new(),
            });
            for dep in &driver.depends_on {
                edges.push(DependencyEdge {
                    id: Uuid::new_v4(),
                    session_id,
                    from: driver_node_id(dep),
                    to: node_id.clone(),
                    kind: DependencyKind::DriverDependency,
                    evidence: Some(format!("'{}' declares dependency on '{}'", driver.name, dep)),
                });
            }
        }

        for module in &normalized.modules {
            if !normalized.processes.contains_key(&module.parent_pid) {
                continue;
            }
            edges.push(DependencyEdge {
                id: Uuid::new_v4(),
                session_id,
                from: process_node_id(module.parent_pid),
                to: format!("module:{}", module.name),
                kind: DependencyKind::LibraryDependency,
                evidence: module.path.clone(),
            });
        }

        DependencyGraph { nodes, edges }
    }

    /// Longest chain (by summed process lifetime) from the kernel root to
    /// any leaf, restricted to `ParentChild` edges since those are
    /// guaranteed to form a tree (no cycles). Returns the set of node ids
    /// on that path.
    pub fn critical_path(graph: &DependencyGraph, normalized: &NormalizedSession) -> HashSet<String> {
        let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
        for edge in &graph.edges {
            if edge.kind == DependencyKind::ParentChild {
                children.entry(edge.from.as_str()).or_default().push(edge.to.as_str());
            }
        }

        let duration_ms = |node_id: &str| -> i64 {
            node_id
                .strip_prefix("process:")
                .and_then(|pid| pid.parse::<u32>().ok())
                .and_then(|pid| normalized.processes.get(&pid))
                .and_then(|p| p.lifetime())
                .map(|d| d.num_milliseconds().max(0))
                .unwrap_or(0)
        };

        // DFS returning (total duration along best path, path node ids).
        fn dfs<'a>(
            node: &'a str,
            children: &HashMap<&'a str, Vec<&'a str>>,
            duration_ms: &dyn Fn(&str) -> i64,
        ) -> (i64, Vec<&'a str>) {
            let own = duration_ms(node);
            match children.get(node) {
                None => (own, vec![node]),
                Some(kids) => {
                    let mut best = (0i64, Vec::new());
                    for kid in kids {
                        let candidate = dfs(kid, children, duration_ms);
                        if candidate.0 > best.0 {
                            best = candidate;
                        }
                    }
                    let mut path = vec![node];
                    path.extend(best.1);
                    (own + best.0, path)
                }
            }
        }

        let (_, path) = dfs(KERNEL_NODE_ID, &children, &duration_ms);
        path.into_iter().map(|s| s.to_string()).collect()
    }
}

pub fn process_node_id(pid: u32) -> String {
    format!("process:{pid}")
}

pub fn service_node_id(name: &str) -> String {
    format!("service:{name}")
}

pub fn driver_node_id(name: &str) -> String {
    format!("driver:{name}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use spm_core::{BootSession, Platform, ProcessInfo};

    fn proc(session: Uuid, pid: u32, ppid: Option<u32>) -> ProcessInfo {
        let mut p = ProcessInfo::new(session, pid, format!("proc{pid}.exe"));
        p.ppid = ppid;
        p
    }

    #[test]
    fn builds_parent_child_edges_and_kernel_root() {
        let session = BootSession::new("host", Platform::Windows, "test");
        let mut normalized = NormalizedSession::default();
        normalized.processes.insert(1, proc(session.id, 1, None));
        normalized.processes.insert(2, proc(session.id, 2, Some(1)));

        let graph = DependencyGraphBuilder::build(session.id, &normalized);

        assert!(graph.nodes.iter().any(|n| n.id == KERNEL_NODE_ID));
        assert!(graph.edges.iter().any(|e| e.from == KERNEL_NODE_ID && e.to == process_node_id(1)));
        assert!(graph.edges.iter().any(|e| e.from == process_node_id(1) && e.to == process_node_id(2)));
    }
}
