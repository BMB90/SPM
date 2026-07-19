import { useState } from "react";
import { api } from "../api/client";
import { SeverityBadge } from "../components/Badges";
import { StatTile } from "../components/StatTile";
import { useSessionContext } from "../context/SessionContext";
import { useAsync } from "../hooks/useAsync";

export function Dashboard() {
  const { currentSessionId, sessions, refreshSessions } = useSessionContext();
  const [capturing, setCapturing] = useState(false);
  const [captureError, setCaptureError] = useState<string | null>(null);

  const session = sessions.find((s) => s.id === currentSessionId) ?? null;

  const processes = useAsync(
    () => (currentSessionId ? api.listProcesses(currentSessionId, { limit: 1000 }) : null),
    [currentSessionId],
  );
  const services = useAsync(() => (currentSessionId ? api.listServices(currentSessionId, { limit: 1 }) : null), [currentSessionId]);
  const drivers = useAsync(() => (currentSessionId ? api.listDrivers(currentSessionId, { limit: 1 }) : null), [currentSessionId]);
  const timeline = useAsync(() => (currentSessionId ? api.getTimeline(currentSessionId) : null), [currentSessionId]);

  const findings = (processes.data?.items ?? [])
    .flatMap((p) => p.security.findings.map((f) => ({ ...f, process: p.executable_name, pid: p.pid })))
    .sort((a, b) => severityRank(b.severity) - severityRank(a.severity))
    .slice(0, 10);

  const bootDurationSeconds = timeline.data && timeline.data.length > 0 ? Math.max(...timeline.data.map((t) => t.offset_seconds)) : null;

  async function runCapture() {
    setCapturing(true);
    setCaptureError(null);
    try {
      await api.startCapture({});
      // Poll for completion, then refresh the session list.
      await new Promise((r) => setTimeout(r, 4000));
      await refreshSessions();
    } catch (e) {
      setCaptureError(e instanceof Error ? e.message : String(e));
    } finally {
      setCapturing(false);
    }
  }

  return (
    <>
      <div className="topbar">
        <h1>Dashboard</h1>
        <div style={{ display: "flex", gap: 10 }}>
          {captureError && <span style={{ color: "var(--status-critical)" }}>{captureError}</span>}
          <button className="btn" onClick={runCapture} disabled={capturing}>
            {capturing ? "Capturing…" : "New Capture"}
          </button>
        </div>
      </div>

      {!session && <div className="empty-state">No boot session captured yet. Click "New Capture" to run one.</div>}

      {session && (
        <>
          <div className="stat-grid">
            <StatTile value={processes.data?.total ?? "…"} label="Processes" />
            <StatTile value={services.data?.total ?? "…"} label="Services" />
            <StatTile value={drivers.data?.total ?? "…"} label="Drivers" />
            <StatTile value={bootDurationSeconds !== null ? `${bootDurationSeconds.toFixed(1)}s` : "—"} label="Boot span captured" />
            <StatTile value={findings.length} label="Security findings (top page)" />
          </div>

          <div className="card">
            <h2>Session</h2>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, fontSize: 13, color: "var(--text-secondary)" }}>
              <div>
                <strong>Host</strong> {session.hostname}
              </div>
              <div>
                <strong>Platform</strong> {session.platform} — {session.os_version}
              </div>
              <div>
                <strong>Captured</strong> {new Date(session.capture_started_at).toLocaleString()}
              </div>
              <div>
                <strong>Status</strong> {session.capture_completed_at ? "complete" : "incomplete"}
              </div>
            </div>
          </div>

          <div className="card">
            <h2>Top Security Findings</h2>
            {findings.length === 0 ? (
              <div className="empty-state">No findings flagged in this capture.</div>
            ) : (
              <table className="data-table">
                <thead>
                  <tr>
                    <th>Severity</th>
                    <th>Process</th>
                    <th>Message</th>
                  </tr>
                </thead>
                <tbody>
                  {findings.map((f, i) => (
                    <tr key={i}>
                      <td>
                        <SeverityBadge severity={f.severity} />
                      </td>
                      <td>
                        {f.process} (pid {f.pid})
                      </td>
                      <td>{f.message}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </>
      )}
    </>
  );
}

function severityRank(s: string): number {
  return { critical: 4, high: 3, medium: 2, low: 1, info: 0 }[s] ?? 0;
}
