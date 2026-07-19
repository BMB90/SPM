import { createColumnHelper } from "@tanstack/react-table";
import { useMemo, useState } from "react";
import { api } from "../api/client";
import { DataTable } from "../components/DataTable";
import { Pagination } from "../components/Pagination";
import { useSessionContext } from "../context/SessionContext";
import { useAsync } from "../hooks/useAsync";
import type { FileActivity } from "../api/types";

const PAGE_SIZE = 300;
const columnHelper = createColumnHelper<FileActivity>();

export function FileActivityPage() {
  const { currentSessionId } = useSessionContext();
  const [offset, setOffset] = useState(0);
  const [filter, setFilter] = useState("");

  const page = useAsync(() => (currentSessionId ? api.listFileActivity(currentSessionId, { limit: PAGE_SIZE, offset }) : null), [
    currentSessionId,
    offset,
  ]);

  const filtered = (page.data?.items ?? []).filter((f) => (filter ? f.path.toLowerCase().includes(filter.toLowerCase()) : true));

  const columns = useMemo(
    () => [
      columnHelper.accessor("timestamp", { header: "Time", cell: (c) => new Date(c.getValue()).toLocaleTimeString() }),
      columnHelper.accessor("operation", { header: "Operation" }),
      columnHelper.accessor("path", { header: "Path" }),
      columnHelper.accessor("process_executable", { header: "Process", cell: (c) => c.getValue() ?? "—" }),
      columnHelper.accessor("pid", { header: "PID" }),
      columnHelper.accessor("owner", { header: "Owner", cell: (c) => c.getValue() ?? "—" }),
    ],
    [],
  );

  return (
    <>
      <div className="topbar">
        <h1>File Activity</h1>
        <input className="search-input" placeholder="Filter by path…" value={filter} onChange={(e) => setFilter(e.target.value)} />
      </div>
      {(page.data?.items.length ?? 0) === 0 && !page.loading && (
        <div className="empty-state">
          No file-activity events for this session. This collector (fanotify/inotify on Linux, Sysmon/ETW file-IO on Windows) is a documented
          extension point — see docs/collector-architecture.md.
        </div>
      )}
      <DataTable columns={columns} data={filtered} emptyMessage={page.loading ? "Loading…" : "No file activity"} />
      {page.data && <Pagination offset={offset} limit={PAGE_SIZE} total={page.data.total} onOffsetChange={setOffset} />}
    </>
  );
}
