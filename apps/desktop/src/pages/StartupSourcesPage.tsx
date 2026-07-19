import { createColumnHelper } from "@tanstack/react-table";
import { useMemo, useState } from "react";
import { api } from "../api/client";
import { DataTable } from "../components/DataTable";
import { Pagination } from "../components/Pagination";
import { useSessionContext } from "../context/SessionContext";
import { useAsync } from "../hooks/useAsync";
import type { ConfigEntry } from "../api/types";

const PAGE_SIZE = 300;
const columnHelper = createColumnHelper<ConfigEntry>();

export function StartupSourcesPage() {
  const { currentSessionId } = useSessionContext();
  const [offset, setOffset] = useState(0);
  const [filter, setFilter] = useState("");

  const page = useAsync(() => (currentSessionId ? api.listConfigEntries(currentSessionId, { limit: PAGE_SIZE, offset }) : null), [
    currentSessionId,
    offset,
  ]);

  const filtered = (page.data?.items ?? []).filter((c) =>
    filter ? `${c.location} ${c.name ?? ""} ${c.value ?? ""}`.toLowerCase().includes(filter.toLowerCase()) : true,
  );

  const columns = useMemo(
    () => [
      columnHelper.accessor("kind", { header: "Kind" }),
      columnHelper.accessor("location", { header: "Location" }),
      columnHelper.accessor("name", { header: "Name", cell: (c) => c.getValue() ?? "—" }),
      columnHelper.accessor("value", { header: "Value", cell: (c) => c.getValue() ?? "—" }),
    ],
    [],
  );

  return (
    <>
      <div className="topbar">
        <h1>Startup Sources</h1>
        <input className="search-input" placeholder="Filter registry keys, tasks, startup items…" value={filter} onChange={(e) => setFilter(e.target.value)} />
      </div>
      <p style={{ color: "var(--text-muted)", fontSize: 12, marginTop: -8 }}>
        Registry Run/RunOnce keys, Startup folder contents, and Scheduled Task definitions — the evidence `spm-analysis`'s startup-source detector
        cross-references against running processes.
      </p>
      <DataTable columns={columns} data={filtered} emptyMessage={page.loading ? "Loading…" : "No startup sources"} />
      {page.data && <Pagination offset={offset} limit={PAGE_SIZE} total={page.data.total} onOffsetChange={setOffset} />}
    </>
  );
}
