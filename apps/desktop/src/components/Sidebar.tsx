import { NavLink } from "react-router-dom";
import { useSessionContext } from "../context/SessionContext";
import { ThemeToggle } from "./ThemeToggle";

const LINKS = [
  { to: "/", label: "Dashboard", end: true },
  { to: "/processes", label: "Processes" },
  { to: "/services", label: "Services" },
  { to: "/drivers", label: "Drivers" },
  { to: "/files", label: "File Activity" },
  { to: "/network", label: "Network" },
  { to: "/startup-sources", label: "Startup Sources" },
  { to: "/timeline", label: "Timeline" },
  { to: "/graph", label: "Dependency Graph" },
  { to: "/compare", label: "Compare Sessions" },
];

export function Sidebar() {
  const { sessions, currentSessionId, setCurrentSessionId } = useSessionContext();

  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        SPM
        <small>Startup Intelligence Platform</small>
      </div>

      <select
        className="filter-select"
        style={{ margin: "0 8px 16px" }}
        value={currentSessionId ?? ""}
        onChange={(e) => setCurrentSessionId(e.target.value)}
      >
        {sessions.length === 0 && <option value="">No sessions</option>}
        {sessions.map((s) => (
          <option key={s.id} value={s.id}>
            {s.hostname} — {new Date(s.capture_started_at).toLocaleString()}
          </option>
        ))}
      </select>

      <nav>
        {LINKS.map((link) => (
          <NavLink key={link.to} to={link.to} end={link.end} className={({ isActive }) => `nav-link${isActive ? " active" : ""}`}>
            {link.label}
          </NavLink>
        ))}
      </nav>

      <div className="sidebar-footer">
        <ThemeToggle />
      </div>
    </aside>
  );
}
