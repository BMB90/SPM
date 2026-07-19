import { useState } from "react";
import { api } from "../api/client";
import { useSessionContext } from "../context/SessionContext";
import { useAsync } from "../hooks/useAsync";

export function ComparePage() {
  const { sessions } = useSessionContext();
  const [baseline, setBaseline] = useState("");
  const [target, setTarget] = useState("");

  const comparison = useAsync(() => (baseline && target ? api.compareSessions(baseline, target) : null), [baseline, target]);

  return (
    <>
      <div className="topbar">
        <h1>Compare Sessions</h1>
      </div>

      <div className="card">
        <div style={{ display: "flex", gap: 16 }}>
          <label style={{ flex: 1 }}>
            Baseline
            <select className="filter-select" style={{ width: "100%", marginTop: 4 }} value={baseline} onChange={(e) => setBaseline(e.target.value)}>
              <option value="">Select a session…</option>
              {sessions.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.hostname} — {new Date(s.capture_started_at).toLocaleString()}
                </option>
              ))}
            </select>
          </label>
          <label style={{ flex: 1 }}>
            Target
            <select className="filter-select" style={{ width: "100%", marginTop: 4 }} value={target} onChange={(e) => setTarget(e.target.value)}>
              <option value="">Select a session…</option>
              {sessions.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.hostname} — {new Date(s.capture_started_at).toLocaleString()}
                </option>
              ))}
            </select>
          </label>
        </div>
      </div>

      {comparison.error && <div className="empty-state">{comparison.error}</div>}

      {comparison.data && (
        <>
          <div className="stat-grid">
            <div className="stat-tile">
              <div className="value">{comparison.data.boot_duration_seconds_delta?.toFixed(2) ?? "—"}s</div>
              <div className="label">Boot duration delta</div>
            </div>
            <div className="stat-tile">
              <div className="value">{comparison.data.processes.added.length}</div>
              <div className="label">Processes added</div>
            </div>
            <div className="stat-tile">
              <div className="value">{comparison.data.processes.removed.length}</div>
              <div className="label">Processes removed</div>
            </div>
            <div className="stat-tile">
              <div className="value">{comparison.data.startup_items.added.length}</div>
              <div className="label">Startup items added</div>
            </div>
            <div className="stat-tile">
              <div className="value">{comparison.data.executable_path_changes.length}</div>
              <div className="label">Executable path changes</div>
            </div>
          </div>

          <div className="card">
            <h2>Executable Path Changes</h2>
            {comparison.data.executable_path_changes.length === 0 ? (
              <div className="empty-state">None detected.</div>
            ) : (
              <table className="data-table">
                <thead>
                  <tr>
                    <th>Executable</th>
                    <th>Old Path</th>
                    <th>New Path</th>
                  </tr>
                </thead>
                <tbody>
                  {comparison.data.executable_path_changes.map((c, i) => (
                    <tr key={i}>
                      <td>{c.executable_name}</td>
                      <td>{c.old_path}</td>
                      <td>{c.new_path}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>

          <div className="card">
            <h2>Startup Item Drift</h2>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
              <div>
                <strong>Added ({comparison.data.startup_items.added.length})</strong>
                <ul>
                  {comparison.data.startup_items.added.map((i) => (
                    <li key={i}>{i}</li>
                  ))}
                </ul>
              </div>
              <div>
                <strong>Removed ({comparison.data.startup_items.removed.length})</strong>
                <ul>
                  {comparison.data.startup_items.removed.map((i) => (
                    <li key={i}>{i}</li>
                  ))}
                </ul>
              </div>
            </div>
          </div>
        </>
      )}
    </>
  );
}
