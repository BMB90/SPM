use rusqlite::params;
use spm_core::{DependencyEdge, DependencyGraph, DependencyKind, GraphNode, NodeKind};
use uuid::Uuid;

use crate::error::StorageResult;

use super::util::{json_from_sql, json_to_sql, uuid_from_sql, uuid_to_sql};

pub fn insert_graph(conn: &mut rusqlite::Connection, session_id: Uuid, graph: &DependencyGraph) -> StorageResult<()> {
    let tx = conn.transaction()?;
    for node in &graph.nodes {
        tx.execute(
            "INSERT INTO graph_nodes (id, session_id, kind, label, attributes_json) VALUES (?1,?2,?3,?4,?5)",
            params![node.id, uuid_to_sql(&session_id), node_kind_to_str(&node.kind), node.label, json_to_sql(&node.attributes)?],
        )?;
    }
    for edge in &graph.edges {
        tx.execute(
            "INSERT INTO graph_edges (id, session_id, from_node, to_node, kind, evidence) VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                uuid_to_sql(&edge.id),
                uuid_to_sql(&edge.session_id),
                edge.from,
                edge.to,
                edge_kind_to_str(&edge.kind),
                edge.evidence,
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn get_graph(conn: &rusqlite::Connection, session_id: Uuid) -> StorageResult<DependencyGraph> {
    let mut node_stmt = conn.prepare("SELECT id, kind, label, attributes_json FROM graph_nodes WHERE session_id = ?1")?;
    let nodes = node_stmt
        .query_map(params![uuid_to_sql(&session_id)], |row| {
            let kind_str: String = row.get(1)?;
            let attrs_json: String = row.get(3)?;
            Ok(GraphNode {
                id: row.get(0)?,
                kind: node_kind_from_str(&kind_str)?,
                label: row.get(2)?,
                attributes: json_from_sql(&attrs_json)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut edge_stmt = conn.prepare("SELECT id, session_id, from_node, to_node, kind, evidence FROM graph_edges WHERE session_id = ?1")?;
    let edges = edge_stmt
        .query_map(params![uuid_to_sql(&session_id)], |row| {
            let kind_str: String = row.get(4)?;
            Ok(DependencyEdge {
                id: uuid_from_sql(row.get(0)?)?,
                session_id: uuid_from_sql(row.get(1)?)?,
                from: row.get(2)?,
                to: row.get(3)?,
                kind: edge_kind_from_str(&kind_str)?,
                evidence: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(DependencyGraph { nodes, edges })
}

fn node_kind_to_str(k: &NodeKind) -> &'static str {
    match k {
        NodeKind::Kernel => "kernel",
        NodeKind::Process => "process",
        NodeKind::Service => "service",
        NodeKind::Driver => "driver",
        NodeKind::Module => "module",
    }
}
fn node_kind_from_str(s: &str) -> rusqlite::Result<NodeKind> {
    Ok(match s {
        "kernel" => NodeKind::Kernel,
        "process" => NodeKind::Process,
        "service" => NodeKind::Service,
        "driver" => NodeKind::Driver,
        _ => NodeKind::Module,
    })
}
fn edge_kind_to_str(k: &DependencyKind) -> &'static str {
    match k {
        DependencyKind::ParentChild => "parent_child",
        DependencyKind::ServiceDependency => "service_dependency",
        DependencyKind::DriverDependency => "driver_dependency",
        DependencyKind::LibraryDependency => "library_dependency",
        DependencyKind::NetworkDependency => "network_dependency",
        DependencyKind::FilesystemDependency => "filesystem_dependency",
        DependencyKind::PackageDependency => "package_dependency",
    }
}
fn edge_kind_from_str(s: &str) -> rusqlite::Result<DependencyKind> {
    Ok(match s {
        "parent_child" => DependencyKind::ParentChild,
        "service_dependency" => DependencyKind::ServiceDependency,
        "driver_dependency" => DependencyKind::DriverDependency,
        "library_dependency" => DependencyKind::LibraryDependency,
        "network_dependency" => DependencyKind::NetworkDependency,
        "filesystem_dependency" => DependencyKind::FilesystemDependency,
        _ => DependencyKind::PackageDependency,
    })
}
