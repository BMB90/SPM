import { createColumnHelper } from "@tanstack/react-table";
import { useMemo, useState } from "react";
import { api } from "../api/client";
import { SignatureBadge } from "../components/Badges";
import { DataTable } from "../components/DataTable";
import { Pagination } from "../components/Pagination";
import { useSessionContext } from "../context/SessionContext";
import { useAsync } from "../hooks/useAsync";
import type { DriverInfo } from "../api/types";

const PAGE_SIZE = 300;
const columnHelper = createColumnHelper<DriverInfo>();

export function DriverExplorer() {
  const { currentSessionId } = useSessionContext();
  const [offset, setOffset] = useState(0);
  const [filter, setFilter] = useState("");

  const page = useAsync(() => (currentSessionId ? api.listDrivers(currentSessionId, { limit: PAGE_SIZE, offset }) : null), [
    currentSessionId,
    offset,
  ]);

  const filtered = (page.data?.items ?? []).filter((d) => (filter ? `${d.name} ${d.path ?? ""}`.toLowerCase().includes(filter.toLowerCase()) : true));

  const columns = useMemo(
    () => [
      columnHelper.accessor("load_order", { header: "#", cell: (c) => c.getValue() ?? "—" }),
      columnHelper.accessor("name", { header: "Name" }),
      columnHelper.accessor("path", { header: "Path", cell: (c) => c.getValue() ?? "—" }),
      columnHelper.accessor("status", { header: "Status" }),
      columnHelper.accessor("signature_status", { header: "Signature", cell: (c) => <SignatureBadge status={c.getValue()} /> }),
      columnHelper.accessor("failure_reason", { header: "Failure", cell: (c) => c.getValue() ?? "—" }),
    ],
    [],
  );

  return (
    <>
      <div className="topbar">
        <h1>Driver Explorer</h1>
        <input className="search-input" placeholder="Filter by name/path…" value={filter} onChange={(e) => setFilter(e.target.value)} />
      </div>
      <DataTable columns={columns} data={filtered} emptyMessage={page.loading ? "Loading…" : "No drivers"} />
      {page.data && <Pagination offset={offset} limit={PAGE_SIZE} total={page.data.total} onOffsetChange={setOffset} />}
    </>
  );
}
