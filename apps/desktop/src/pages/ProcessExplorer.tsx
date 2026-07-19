import { createColumnHelper } from "@tanstack/react-table";
import { useMemo, useState } from "react";
import { api } from "../api/client";
import { RoleBadge, SignatureBadge } from "../components/Badges";
import { DataTable } from "../components/DataTable";
import { Pagination } from "../components/Pagination";
import { useSessionContext } from "../context/SessionContext";
import { useAsync } from "../hooks/useAsync";
import type { ProcessInfo, ProcessRole } from "../api/types";

const PAGE_SIZE = 200;

const ROLE_OPTIONS: { value: ProcessRole | ""; label: string }[] = [
  { value: "", label: "All roles" },
  { value: "kernel_process", label: "Kernel process" },
  { value: "system", label: "System" },
  { value: "service", label: "Service" },
  { value: "daemon", label: "Daemon" },
  { value: "scheduled_task", label: "Scheduled task" },
  { value: "login_item", label: "Login item" },
  { value: "user_application", label: "User application" },
  { value: "unknown", label: "Unknown" },
];

const columnHelper = createColumnHelper<ProcessInfo>();

function formatStartupSource(p: ProcessInfo): string {
  const src = p.startup_source;
  if (!src) return "—";
  const kind = src.kind;
  switch (kind.kind) {
    case "registry_run_key":
      return `Registry: ${kind.hive}\\${kind.key}\\${kind.value}`;
    case "startup_folder":
      return `Startup folder: ${kind.path}`;
    case "scheduled_task":
      return `Scheduled task: ${kind.task_path}`;
    case "windows_service":
      return `Service: ${kind.service_name}`;
    case "parent_process":
      return `Parent: ${kind.parent_executable ?? kind.parent_pid}`;
    default:
      return kind.kind.replace(/_/g, " ");
  }
}

export function ProcessExplorer() {
  const { currentSessionId } = useSessionContext();
  const [offset, setOffset] = useState(0);
  const [query, setQuery] = useState("");
  const [role, setRole] = useState<string>("");
  const [signedOnly, setSignedOnly] = useState<string>("");
  const [selected, setSelected] = useState<ProcessInfo | null>(null);

  const page = useAsync(() => {
    if (!currentSessionId) return null;
    if (query.trim()) {
      return api.searchProcesses(currentSessionId, query.trim(), { limit: PAGE_SIZE, offset });
    }
    return api.listProcesses(currentSessionId, {
      limit: PAGE_SIZE,
      offset,
      role: role || undefined,
      signed: signedOnly === "" ? undefined : signedOnly === "true",
    });
  }, [currentSessionId, offset, query, role, signedOnly]);

  const columns = useMemo(
    () => [
      columnHelper.accessor("pid", { header: "PID" }),
      columnHelper.accessor("ppid", { header: "PPID", cell: (c) => c.getValue() ?? "—" }),
      columnHelper.accessor("executable_name", { header: "Name" }),
      columnHelper.accessor("executable_path", { header: "Path", cell: (c) => c.getValue() ?? "—" }),
      columnHelper.accessor("user", { header: "User", cell: (c) => c.getValue() ?? "—" }),
      columnHelper.accessor("role", { header: "Role", cell: (c) => <RoleBadge role={c.getValue()} /> }),
      columnHelper.accessor("signature_status", { header: "Signature", cell: (c) => <SignatureBadge status={c.getValue()} /> }),
      columnHelper.accessor((p) => p.performance.cpu_percent_peak ?? 0, {
        id: "cpu",
        header: "CPU %",
        cell: (c) => c.getValue().toFixed(1),
      }),
      columnHelper.accessor((p) => p.performance.memory_bytes_current ?? 0, {
        id: "memory",
        header: "Memory",
        cell: (c) => formatBytes(c.getValue()),
      }),
      columnHelper.accessor((p) => formatStartupSource(p), { id: "startup_source", header: "Startup Source" }),
    ],
    [],
  );

  return (
    <>
      <div className="topbar">
        <h1>Process Explorer</h1>
        <div style={{ display: "flex", gap: 8 }}>
          <input
            className="search-input"
            placeholder="Search name, path, pid, sha256, command line…"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setOffset(0);
            }}
          />
          <select
            className="filter-select"
            value={role}
            onChange={(e) => {
              setRole(e.target.value);
              setOffset(0);
            }}
          >
            {ROLE_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
          <select
            className="filter-select"
            value={signedOnly}
            onChange={(e) => {
              setSignedOnly(e.target.value);
              setOffset(0);
            }}
          >
            <option value="">Signed + unsigned</option>
            <option value="true">Signed only</option>
            <option value="false">Unsigned only</option>
          </select>
        </div>
      </div>

      {page.error && <div className="empty-state">{page.error}</div>}

      <DataTable columns={columns} data={page.data?.items ?? []} onRowClick={setSelected} emptyMessage={page.loading ? "Loading…" : "No processes"} />

      {page.data && <Pagination offset={offset} limit={PAGE_SIZE} total={page.data.total} onOffsetChange={setOffset} />}

      {selected && <ProcessDetail process={selected} onClose={() => setSelected(null)} />}
    </>
  );
}

function ProcessDetail({ process, onClose }: { process: ProcessInfo; onClose: () => void }) {
  return (
    <div className="card" style={{ marginTop: 16 }}>
      <div style={{ display: "flex", justifyContent: "space-between" }}>
        <h2>
          {process.executable_name} (pid {process.pid})
        </h2>
        <button className="btn secondary" onClick={onClose}>
          Close
        </button>
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, fontSize: 13 }}>
        <div>
          <strong>Path</strong> {process.executable_path ?? "—"}
        </div>
        <div>
          <strong>Command line</strong> {process.command_line ?? "—"}
        </div>
        <div>
          <strong>Working directory</strong> {process.working_directory ?? "—"}
        </div>
        <div>
          <strong>User</strong> {process.user ?? "—"}
        </div>
        <div>
          <strong>SHA-256</strong> {process.sha256 ?? "—"}
        </div>
        <div>
          <strong>Signer</strong> {process.signer ?? "—"}
        </div>
        <div>
          <strong>Start time</strong> {process.start_time ? new Date(process.start_time).toLocaleString() : "—"}
        </div>
        <div>
          <strong>Startup source</strong> {formatStartupSource(process)}
        </div>
      </div>
      {process.security.findings.length > 0 && (
        <>
          <h2 style={{ marginTop: 14 }}>Findings</h2>
          <ul>
            {process.security.findings.map((f, i) => (
              <li key={i}>
                [{f.severity}] {f.message}
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "—";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(1)} ${units[unit]}`;
}
