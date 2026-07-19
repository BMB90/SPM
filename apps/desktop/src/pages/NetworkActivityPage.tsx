import { createColumnHelper } from "@tanstack/react-table";
import { useMemo, useState } from "react";
import { api } from "../api/client";
import { DataTable } from "../components/DataTable";
import { Pagination } from "../components/Pagination";
import { useSessionContext } from "../context/SessionContext";
import { useAsync } from "../hooks/useAsync";
import type { NetworkActivity } from "../api/types";

const PAGE_SIZE = 300;
const columnHelper = createColumnHelper<NetworkActivity>();

export function NetworkActivityPage() {
  const { currentSessionId } = useSessionContext();
  const [offset, setOffset] = useState(0);

  const page = useAsync(() => (currentSessionId ? api.listNetworkActivity(currentSessionId, { limit: PAGE_SIZE, offset }) : null), [
    currentSessionId,
    offset,
  ]);

  const columns = useMemo(
    () => [
      columnHelper.accessor("started_at", { header: "Time", cell: (c) => new Date(c.getValue()).toLocaleTimeString() }),
      columnHelper.accessor("protocol", { header: "Protocol" }),
      columnHelper.accessor("process_executable", { header: "Process", cell: (c) => c.getValue() ?? "—" }),
      columnHelper.accessor("pid", { header: "PID" }),
      columnHelper.accessor("remote_address", { header: "Remote", cell: (c) => c.getValue() ?? "—" }),
      columnHelper.accessor("remote_port", { header: "Port", cell: (c) => c.getValue() ?? "—" }),
      columnHelper.accessor("dns_query", { header: "DNS Query", cell: (c) => c.getValue() ?? "—" }),
    ],
    [],
  );

  return (
    <>
      <div className="topbar">
        <h1>Network Activity</h1>
      </div>
      {(page.data?.items.length ?? 0) === 0 && !page.loading && (
        <div className="empty-state">
          No network events for this session yet — the network collector (per-process TCP/UDP + DNS correlation) is a documented extension point;
          see docs/collector-architecture.md.
        </div>
      )}
      <DataTable columns={columns} data={page.data?.items ?? []} emptyMessage={page.loading ? "Loading…" : "No network activity"} />
      {page.data && <Pagination offset={offset} limit={PAGE_SIZE} total={page.data.total} onOffsetChange={setOffset} />}
    </>
  );
}
