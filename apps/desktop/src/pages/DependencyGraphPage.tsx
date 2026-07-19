import { useMemo, useState } from "react";
import ReactFlow, { Background, Controls, MiniMap, type Edge, type Node } from "reactflow";
import "reactflow/dist/style.css";
import { api } from "../api/client";
import { useSessionContext } from "../context/SessionContext";
import { useAsync } from "../hooks/useAsync";
import type { DependencyGraph, DependencyKind, NodeKind } from "../api/types";

const KIND_COLOR: Record<NodeKind, string> = {
  kernel: "#4a3aa7",
  process: "#2a78d6",
  service: "#008300",
  driver: "#eb6834",
  module: "#898781",
};

const EDGE_COLOR: Record<DependencyKind, string> = {
  parent_child: "#2a78d6",
  service_dependency: "#008300",
  driver_dependency: "#eb6834",
  library_dependency: "#898781",
  network_dependency: "#1baf7a",
  filesystem_dependency: "#e87ba4",
  package_dependency: "#eda100",
};

function layout(graph: DependencyGraph): { nodes: Node[]; edges: Edge[] } {
  const childrenByParent = new Map<string, string[]>();
  const hasIncoming = new Set<string>();
  for (const edge of graph.edges) {
    if (!childrenByParent.has(edge.from)) childrenByParent.set(edge.from, []);
    childrenByParent.get(edge.from)!.push(edge.to);
    hasIncoming.add(edge.to);
  }

  const roots = graph.nodes.filter((n) => !hasIncoming.has(n.id));
  const depth = new Map<string, number>();
  const order: string[] = [];
  const queue: string[] = roots.map((r) => r.id);
  roots.forEach((r) => depth.set(r.id, 0));

  while (queue.length > 0) {
    const id = queue.shift()!;
    if (order.includes(id)) continue;
    order.push(id);
    const d = depth.get(id) ?? 0;
    for (const child of childrenByParent.get(id) ?? []) {
      if (!depth.has(child) || (depth.get(child) ?? 0) < d + 1) {
        depth.set(child, d + 1);
      }
      queue.push(child);
    }
  }

  const columnCounters = new Map<number, number>();
  const nodes: Node[] = graph.nodes.map((n) => {
    const d = depth.get(n.id) ?? 0;
    const col = columnCounters.get(d) ?? 0;
    columnCounters.set(d, col + 1);
    return {
      id: n.id,
      position: { x: col * 220, y: d * 110 },
      data: { label: n.label },
      style: {
        background: KIND_COLOR[n.kind] ?? "#666",
        color: "white",
        border: "none",
        borderRadius: 6,
        fontSize: 11,
        padding: 8,
        width: 180,
      },
    };
  });

  const edges: Edge[] = graph.edges.map((e) => ({
    id: e.id,
    source: e.from,
    target: e.to,
    label: e.kind === "parent_child" ? undefined : e.kind.replace(/_/g, " "),
    style: { stroke: EDGE_COLOR[e.kind] ?? "#666" },
    animated: false,
  }));

  return { nodes, edges };
}

export function DependencyGraphPage() {
  const { currentSessionId } = useSessionContext();
  const [kindFilter, setKindFilter] = useState("");
  const graph = useAsync(() => (currentSessionId ? api.getGraph(currentSessionId) : null), [currentSessionId]);

  const { nodes, edges } = useMemo(() => {
    if (!graph.data) return { nodes: [], edges: [] };
    const filteredGraph: DependencyGraph = kindFilter
      ? {
          nodes: graph.data.nodes.filter((n) => n.kind === kindFilter || n.kind === "kernel"),
          edges: graph.data.edges,
        }
      : graph.data;
    return layout(filteredGraph);
  }, [graph.data, kindFilter]);

  return (
    <>
      <div className="topbar">
        <h1>Dependency Graph</h1>
        <select className="filter-select" value={kindFilter} onChange={(e) => setKindFilter(e.target.value)}>
          <option value="">All node kinds</option>
          <option value="process">Processes</option>
          <option value="service">Services</option>
          <option value="driver">Drivers</option>
          <option value="module">Modules</option>
        </select>
      </div>

      <div className="legend">
        {Object.entries(KIND_COLOR).map(([kind, color]) => (
          <span key={kind}>
            <span className="legend-swatch" style={{ background: color }} />
            {kind}
          </span>
        ))}
      </div>

      {nodes.length === 0 ? (
        <div className="empty-state">{graph.loading ? "Loading…" : "No graph data for this session."}</div>
      ) : (
        <div style={{ height: "calc(100vh - 260px)", border: "1px solid var(--border)", borderRadius: 8 }}>
          <ReactFlow nodes={nodes} edges={edges} fitView>
            <Background />
            <Controls />
            <MiniMap />
          </ReactFlow>
        </div>
      )}
    </>
  );
}
