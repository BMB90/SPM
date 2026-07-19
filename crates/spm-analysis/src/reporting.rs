use serde::Serialize;
use spm_core::{BootSession, ConfigEntry, DependencyGraph, DriverInfo, FileActivity, NetworkActivity, ProcessInfo, ServiceInfo, TimelineEntry};
use spm_storage::{Pagination, ProcessFilter, Storage, StorageResult};
use uuid::Uuid;

const EXPORT_PAGE_SIZE: u32 = 20_000;

/// Everything captured for one session, denormalized into a single
/// structure — the machine-readable schema backing every export format
/// (JSON is this struct verbatim; CSV/Markdown/HTML are derived views).
#[derive(Debug, Clone, Serialize)]
pub struct SessionReport {
    pub session: BootSession,
    pub processes: Vec<ProcessInfo>,
    pub services: Vec<ServiceInfo>,
    pub drivers: Vec<DriverInfo>,
    pub file_activity: Vec<FileActivity>,
    pub network_activity: Vec<NetworkActivity>,
    pub config_entries: Vec<ConfigEntry>,
    pub timeline: Vec<TimelineEntry>,
    pub graph: DependencyGraph,
}

pub struct ReportGenerator<'a> {
    storage: &'a Storage,
}

impl<'a> ReportGenerator<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    pub fn load(&self, session_id: Uuid) -> StorageResult<SessionReport> {
        let page = Pagination::new(EXPORT_PAGE_SIZE, 0);
        Ok(SessionReport {
            session: self.storage.get_session(session_id)?,
            processes: self.storage.list_processes(session_id, &ProcessFilter::default(), page)?.items,
            services: self.storage.list_services(session_id, page)?.items,
            drivers: self.storage.list_drivers(session_id, page)?.items,
            file_activity: self.storage.list_file_activity(session_id, page)?.items,
            network_activity: self.storage.list_network_activity(session_id, page)?.items,
            config_entries: self.storage.list_config_entries(session_id, page)?.items,
            timeline: self.storage.get_timeline(session_id)?,
            graph: self.storage.get_graph(session_id)?,
        })
    }

    pub fn to_json(&self, session_id: Uuid) -> StorageResult<String> {
        let report = self.load(session_id)?;
        Ok(serde_json::to_string_pretty(&report).unwrap_or_default())
    }

    pub fn to_csv_processes(&self, session_id: Uuid) -> StorageResult<String> {
        let report = self.load(session_id)?;
        let mut writer = csv::Writer::from_writer(Vec::new());
        writer
            .write_record([
                "pid", "ppid", "name", "path", "user", "role", "signature_status", "sha256", "start_time",
                "exit_time", "startup_source",
            ])
            .ok();
        for p in &report.processes {
            let startup = p
                .startup_source
                .as_ref()
                .map(|s| format!("{:?}", s.kind))
                .unwrap_or_default();
            writer
                .write_record([
                    p.pid.to_string(),
                    p.ppid.map(|v| v.to_string()).unwrap_or_default(),
                    p.executable_name.clone(),
                    p.executable_path.clone().unwrap_or_default(),
                    p.user.clone().unwrap_or_default(),
                    format!("{:?}", p.role),
                    format!("{:?}", p.signature_status),
                    p.sha256.clone().unwrap_or_default(),
                    p.start_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
                    p.exit_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
                    startup,
                ])
                .ok();
        }
        let bytes = writer.into_inner().unwrap_or_default();
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    pub fn to_markdown(&self, session_id: Uuid) -> StorageResult<String> {
        let report = self.load(session_id)?;
        let mut out = String::new();
        out.push_str(&format!("# Boot Report — {}\n\n", report.session.hostname));
        out.push_str(&format!("- **Session ID**: {}\n", report.session.id));
        out.push_str(&format!("- **Platform**: {} ({})\n", report.session.platform, report.session.os_version));
        out.push_str(&format!("- **Capture started**: {}\n", report.session.capture_started_at.to_rfc3339()));
        if let Some(completed) = report.session.capture_completed_at {
            out.push_str(&format!("- **Capture completed**: {}\n", completed.to_rfc3339()));
        }
        out.push_str(&format!("\n## Summary\n\n"));
        out.push_str(&format!("- Processes: {}\n", report.processes.len()));
        out.push_str(&format!("- Services: {}\n", report.services.len()));
        out.push_str(&format!("- Drivers: {}\n", report.drivers.len()));
        out.push_str(&format!("- Config entries (startup sources): {}\n", report.config_entries.len()));
        out.push_str(&format!("- Timeline entries: {}\n", report.timeline.len()));

        let flagged: Vec<_> = report.processes.iter().filter(|p| !p.security.findings.is_empty()).collect();
        out.push_str(&format!("\n## Security Findings ({})\n\n", flagged.len()));
        for p in flagged {
            for finding in &p.security.findings {
                out.push_str(&format!(
                    "- **[{:?}]** `{}` (pid {}): {}\n",
                    finding.severity, p.executable_name, p.pid, finding.message
                ));
            }
        }

        out.push_str("\n## Top-level Timeline\n\n");
        out.push_str("| Offset (s) | Stage | Label | Duration (ms) |\n|---|---|---|---|\n");
        for entry in report.timeline.iter().take(200) {
            out.push_str(&format!(
                "| {:.2} | {} | {} | {} |\n",
                entry.offset_seconds,
                entry.stage,
                entry.label,
                entry.duration_ms.map(|d| d.to_string()).unwrap_or_default()
            ));
        }

        Ok(out)
    }

    /// Self-contained HTML report. Deliberately simple/print-friendly —
    /// "PDF-ready" here means "renders cleanly via a browser's Print to
    /// PDF", not that this crate embeds a PDF renderer.
    pub fn to_html(&self, session_id: Uuid) -> StorageResult<String> {
        let report = self.load(session_id)?;
        let mut rows = String::new();
        for p in report.processes.iter().take(2000) {
            rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                p.pid,
                html_escape(&p.executable_name),
                html_escape(p.executable_path.as_deref().unwrap_or("")),
                html_escape(p.user.as_deref().unwrap_or("")),
                format!("{:?}", p.role),
            ));
        }
        Ok(format!(
            r#"<!doctype html><html><head><meta charset="utf-8"><title>SPM Boot Report — {host}</title>
<style>
body {{ font-family: -apple-system, Segoe UI, sans-serif; margin: 2rem; color: #1a1a1a; }}
table {{ border-collapse: collapse; width: 100%; margin-top: 1rem; }}
th, td {{ border: 1px solid #ddd; padding: 6px 10px; font-size: 13px; text-align: left; }}
th {{ background: #f4f4f4; }}
h1 {{ margin-bottom: 0; }}
.meta {{ color: #555; margin-bottom: 1.5rem; }}
</style></head><body>
<h1>Boot Report — {host}</h1>
<div class="meta">Session {session_id} · {platform} · captured {captured}</div>
<h2>Processes ({count})</h2>
<table><thead><tr><th>PID</th><th>Name</th><th>Path</th><th>User</th><th>Role</th></tr></thead><tbody>
{rows}
</tbody></table>
</body></html>"#,
            host = html_escape(&report.session.hostname),
            session_id = report.session.id,
            platform = report.session.platform,
            captured = report.session.capture_started_at.to_rfc3339(),
            count = report.processes.len(),
            rows = rows,
        ))
    }

    /// Copies this session's data into a brand-new standalone SQLite
    /// database at `dest_path`, reusing the same schema/repositories —
    /// the resulting file is a fully self-contained `spm-storage` database
    /// that can be opened by any SPM tool.
    pub fn export_sqlite(&self, session_id: Uuid, dest_path: &std::path::Path) -> StorageResult<()> {
        let report = self.load(session_id)?;
        let dest = Storage::open(dest_path)?;
        dest.create_session(&report.session)?;
        if !report.processes.is_empty() {
            dest.save_processes(&report.processes)?;
        }
        if !report.services.is_empty() {
            dest.save_services(&report.services)?;
        }
        if !report.drivers.is_empty() {
            dest.save_drivers(&report.drivers)?;
        }
        if !report.file_activity.is_empty() {
            dest.save_file_activity(&report.file_activity)?;
        }
        if !report.network_activity.is_empty() {
            dest.save_network_activity(&report.network_activity)?;
        }
        if !report.config_entries.is_empty() {
            dest.save_config_entries(&report.config_entries)?;
        }
        if !report.timeline.is_empty() {
            dest.save_timeline(&report.timeline)?;
        }
        dest.save_graph(session_id, &report.graph)?;
        Ok(())
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
