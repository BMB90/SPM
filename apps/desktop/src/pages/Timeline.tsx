import { useMemo, useState } from "react";
import { api } from "../api/client";
import { useSessionContext } from "../context/SessionContext";
import { useAsync } from "../hooks/useAsync";
import type { BootStage, TimelineEntry } from "../api/types";

const STAGE_COLORS: Record<string, string> = {
  firmware: "var(--series-7)",
  bootloader: "var(--series-7)",
  kernel: "var(--series-1)",
  driver_init: "var(--series-6)",
  filesystem_mount: "var(--series-5)",
  device_discovery: "var(--series-5)",
  service_startup: "var(--series-2)",
  network_init: "var(--series-5)",
  login_manager: "var(--series-3)",
  user_login: "var(--series-3)",
  desktop_init: "var(--series-4)",
  startup_applications: "var(--series-4)",
  scheduled_tasks: "var(--series-6)",
  background_daemons: "var(--series-2)",
  desktop_ready: "var(--series-4)",
  idle: "var(--text-muted)",
  unknown: "var(--text-muted)",
};

const STAGE_LABELS: Record<string, string> = {
  firmware: "Firmware",
  bootloader: "Bootloader",
  kernel: "Kernel",
  driver_init: "Driver Init",
  filesystem_mount: "Filesystem Mount",
  device_discovery: "Device Discovery",
  service_startup: "Service Startup",
  network_init: "Network Init",
  login_manager: "Login Manager",
  user_login: "User Login",
  desktop_init: "Desktop Init",
  startup_applications: "Startup Applications",
  scheduled_tasks: "Scheduled Tasks",
  background_daemons: "Background Daemons",
  desktop_ready: "Desktop Ready",
  idle: "Idle",
  unknown: "Unknown",
};

export function Timeline() {
  const { currentSessionId } = useSessionContext();
  const [search, setSearch] = useState("");
  const [stageFilter, setStageFilter] = useState<string>("");
  const [pxPerSecond, setPxPerSecond] = useState(40);

  const timeline = useAsync(() => (currentSessionId ? api.getTimeline(currentSessionId) : null), [currentSessionId]);

  const entries = timeline.data ?? [];
  const filtered = entries.filter((e) => {
    if (stageFilter && e.stage !== stageFilter) return false;
    if (search && !e.label.toLowerCase().includes(search.toLowerCase())) return false;
    return true;
  });

  const maxOffset = useMemo(() => Math.max(1, ...entries.map((e) => e.offset_seconds + (e.duration_ms ?? 0) / 1000)), [entries]);
  const usedStages = useMemo(() => Array.from(new Set(entries.map((e) => e.stage))), [entries]);
  const ticks = useMemo(() => buildTicks(maxOffset), [maxOffset]);

  return (
    <>
      <div className="topbar">
        <h1>Boot Timeline</h1>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <input className="search-input" placeholder="Search timeline labels…" value={search} onChange={(e) => setSearch(e.target.value)} />
          <select className="filter-select" value={stageFilter} onChange={(e) => setStageFilter(e.target.value)}>
            <option value="">All stages</option>
            {usedStages.map((s) => (
              <option key={s} value={s}>
                {STAGE_LABELS[s] ?? s}
              </option>
            ))}
          </select>
          <label style={{ fontSize: 12, color: "var(--text-secondary)", display: "flex", gap: 6, alignItems: "center" }}>
            Zoom
            <input type="range" min={10} max={200} value={pxPerSecond} onChange={(e) => setPxPerSecond(Number(e.target.value))} />
          </label>
        </div>
      </div>

      <div className="legend">
        <span>
          <span className="legend-swatch" style={{ background: "var(--status-critical)" }} />
          Critical path
        </span>
        {usedStages.slice(0, 8).map((s) => (
          <span key={s}>
            <span className="legend-swatch" style={{ background: STAGE_COLORS[s] ?? "var(--text-muted)" }} />
            {STAGE_LABELS[s] ?? s}
          </span>
        ))}
      </div>

      {filtered.length === 0 ? (
        <div className="empty-state">{timeline.loading ? "Loading…" : "No timeline entries for this session/filter."}</div>
      ) : (
        <div style={{ overflowX: "auto" }}>
          <div style={{ minWidth: maxOffset * pxPerSecond + 260 }}>
            {filtered.map((entry) => (
              <TimelineRow key={entry.id} entry={entry} pxPerSecond={pxPerSecond} />
            ))}
            <div className="timeline-axis" style={{ width: maxOffset * pxPerSecond }}>
              {ticks.map((t) => (
                <span key={t}>{t}s</span>
              ))}
            </div>
          </div>
        </div>
      )}
    </>
  );
}

function TimelineRow({ entry, pxPerSecond }: { entry: TimelineEntry; pxPerSecond: number }) {
  const left = entry.offset_seconds * pxPerSecond;
  const width = Math.max(3, ((entry.duration_ms ?? 80) / 1000) * pxPerSecond);
  const color = STAGE_COLORS[entry.stage as BootStage] ?? "var(--series-1)";

  return (
    <div className="timeline-row" title={`${entry.label} — +${entry.offset_seconds.toFixed(2)}s`}>
      <div className="timeline-label">{entry.label}</div>
      <div className="timeline-track" style={{ width: Math.max(200, left + width + 40) }}>
        <div
          className={`timeline-bar${entry.on_critical_path ? " critical" : ""}`}
          style={{ left, width, background: entry.on_critical_path ? undefined : color }}
        />
      </div>
    </div>
  );
}

function buildTicks(maxSeconds: number): number[] {
  const step = maxSeconds <= 10 ? 1 : maxSeconds <= 30 ? 5 : maxSeconds <= 120 ? 10 : 30;
  const ticks = [];
  for (let t = 0; t <= maxSeconds; t += step) ticks.push(t);
  return ticks;
}
