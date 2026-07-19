import { createColumnHelper } from "@tanstack/react-table";
import { useMemo, useState } from "react";
import { api } from "../api/client";
import { DataTable } from "../components/DataTable";
import { Pagination } from "../components/Pagination";
import { useSessionContext } from "../context/SessionContext";
import { useAsync } from "../hooks/useAsync";
import type { ServiceInfo } from "../api/types";

const PAGE_SIZE = 200;
const columnHelper = createColumnHelper<ServiceInfo>();

export function ServiceExplorer() {
  const { currentSessionId } = useSessionContext();
  const [offset, setOffset] = useState(0);
  const [filter, setFilter] = useState("");

  const page = useAsync(
    () => (currentSessionId ? api.listServices(currentSessionId, { limit: PAGE_SIZE, offset }) : null),
    [currentSessionId, offset],
  );

  const filtered = (page.data?.items ?? []).filter((s) =>
    filter ? `${s.name} ${s.display_name ?? ""} ${s.binary_path ?? ""}`.toLowerCase().includes(filter.toLowerCase()) : true,
  );

  const columns = useMemo(
    () => [
      columnHelper.accessor("name", { header: "Name" }),
      columnHelper.accessor("display_name", { header: "Display Name", cell: (c) => c.getValue() ?? "—" }),
      columnHelper.accessor("state", { header: "State" }),
      columnHelper.accessor("start_type", { header: "Start Type" }),
      columnHelper.accessor("binary_path", { header: "Binary Path", cell: (c) => c.getValue() ?? "—" }),
      columnHelper.accessor("owner", { header: "Owner", cell: (c) => c.getValue() ?? "—" }),
      columnHelper.accessor("pid", { header: "PID", cell: (c) => c.getValue() ?? "—" }),
      columnHelper.accessor((s) => s.depends_on.join(", "), { id: "depends_on", header: "Depends On" }),
    ],
    [],
  );

  return (
    <>
      <div className="topbar">
        <h1>Service Explorer</h1>
        <input className="search-input" placeholder="Filter by name/path…" value={filter} onChange={(e) => setFilter(e.target.value)} />
      </div>
      <DataTable columns={columns} data={filtered} emptyMessage={page.loading ? "Loading…" : "No services"} />
      {page.data && <Pagination offset={offset} limit={PAGE_SIZE} total={page.data.total} onOffsetChange={setOffset} />}
    </>
  );
}
