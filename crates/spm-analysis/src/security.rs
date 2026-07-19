use std::collections::HashMap;

use spm_core::{FindingSeverity, ProcessRole, SecurityFinding, SignatureStatus, StartupSourceKind};
use spm_engine::NormalizedSession;

const SUSPICIOUS_PATH_FRAGMENTS: &[&str] =
    &[r"\appdata\local\temp\", r"\downloads\", r"\users\public\", r"\programdata\temp\", r"\windows\temp\"];

const OFFICE_PARENTS: &[&str] = &["winword.exe", "excel.exe", "powerpnt.exe", "outlook.exe", "mspub.exe"];
const LOLBINS: &[&str] = &["cmd.exe", "powershell.exe", "pwsh.exe", "wscript.exe", "cscript.exe", "mshta.exe", "rundll32.exe", "regsvr32.exe"];

/// Suspicious-activity heuristics: unsigned binaries persisted from
/// commonly-abused paths, unsigned persistence entries, and known
/// LOLBin-spawned-by-Office parent/child anomalies. These populate
/// `ProcessInfo::security.findings` — the UI surfaces them directly, and
/// `spm-api` can filter/sort on severity.
pub struct SecurityAnalyzer;

impl SecurityAnalyzer {
    pub fn analyze(normalized: &mut NormalizedSession) {
        let parent_names: HashMap<u32, String> =
            normalized.processes.values().map(|p| (p.pid, p.executable_name.to_lowercase())).collect();

        for process in normalized.processes.values_mut() {
            let mut findings = Vec::new();
            let name_lower = process.executable_name.to_lowercase();
            let unsigned = process.signature_status == SignatureStatus::Unsigned;

            if unsigned {
                if let Some(path) = &process.executable_path {
                    let path_lower = path.to_lowercase();
                    if SUSPICIOUS_PATH_FRAGMENTS.iter().any(|f| path_lower.contains(f)) {
                        findings.push(SecurityFinding {
                            severity: FindingSeverity::High,
                            code: "unsigned_exec_from_suspicious_path".to_string(),
                            message: format!("Unsigned executable running from a commonly-abused directory: {path}"),
                        });
                    }
                }

                let is_persistence = process
                    .startup_source
                    .as_ref()
                    .map(|s| {
                        matches!(
                            s.kind,
                            StartupSourceKind::RegistryRunKey { .. }
                                | StartupSourceKind::StartupFolder { .. }
                                | StartupSourceKind::ScheduledTask { .. }
                        )
                    })
                    .unwrap_or(false);
                if is_persistence {
                    findings.push(SecurityFinding {
                        severity: FindingSeverity::Medium,
                        code: "unsigned_persistence".to_string(),
                        message: "Unsigned executable configured as a persistent startup item".to_string(),
                    });
                }
            }

            if let Some(ppid) = process.ppid {
                if let Some(parent_name) = parent_names.get(&ppid) {
                    if OFFICE_PARENTS.contains(&parent_name.as_str()) && LOLBINS.contains(&name_lower.as_str()) {
                        findings.push(SecurityFinding {
                            severity: FindingSeverity::Critical,
                            code: "office_spawned_lolbin".to_string(),
                            message: format!(
                                "{parent_name} spawned {} — a common macro/exploit persistence pattern",
                                process.executable_name
                            ),
                        });
                    }
                }
            }

            if process.role == ProcessRole::KernelProcess && process.ppid.is_some() && process.ppid != Some(0) {
                findings.push(SecurityFinding {
                    severity: FindingSeverity::High,
                    code: "kernel_process_unexpected_parent".to_string(),
                    message: "Process classified as a kernel process has a non-kernel parent".to_string(),
                });
            }

            process.security.findings = findings;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spm_core::{BootSession, Platform, ProcessInfo, StartupSource};
    use spm_engine::NormalizedSession;

    fn session_id() -> uuid::Uuid {
        BootSession::new("host", Platform::Windows, "test").id
    }

    #[test]
    fn flags_unsigned_binary_in_temp_directory() {
        let session_id = session_id();
        let mut normalized = NormalizedSession::default();
        let mut proc = ProcessInfo::new(session_id, 42, "shady.exe");
        proc.executable_path = Some(r"C:\Users\alice\AppData\Local\Temp\shady.exe".to_string());
        proc.signature_status = SignatureStatus::Unsigned;
        normalized.processes.insert(42, proc);

        SecurityAnalyzer::analyze(&mut normalized);

        let findings = &normalized.processes[&42].security.findings;
        assert!(findings.iter().any(|f| f.code == "unsigned_exec_from_suspicious_path"));
    }

    #[test]
    fn flags_unsigned_persistent_startup_item() {
        let session_id = session_id();
        let mut normalized = NormalizedSession::default();
        let mut proc = ProcessInfo::new(session_id, 43, "persist.exe");
        proc.executable_path = Some(r"C:\Program Files\App\persist.exe".to_string());
        proc.signature_status = SignatureStatus::Unsigned;
        proc.startup_source = Some(StartupSource::new(StartupSourceKind::RegistryRunKey {
            hive: "HKLM".to_string(),
            key: "Run".to_string(),
            value: "App".to_string(),
        }));
        normalized.processes.insert(43, proc);

        SecurityAnalyzer::analyze(&mut normalized);

        let findings = &normalized.processes[&43].security.findings;
        assert!(findings.iter().any(|f| f.code == "unsigned_persistence"));
    }

    #[test]
    fn flags_office_spawning_a_lolbin() {
        let session_id = session_id();
        let mut normalized = NormalizedSession::default();

        let mut winword = ProcessInfo::new(session_id, 10, "winword.exe");
        winword.executable_path = Some(r"C:\Program Files\Microsoft Office\WINWORD.EXE".to_string());
        normalized.processes.insert(10, winword);

        let mut powershell = ProcessInfo::new(session_id, 11, "powershell.exe");
        powershell.ppid = Some(10);
        normalized.processes.insert(11, powershell);

        SecurityAnalyzer::analyze(&mut normalized);

        let findings = &normalized.processes[&11].security.findings;
        assert!(findings.iter().any(|f| f.code == "office_spawned_lolbin" && f.severity == FindingSeverity::Critical));
    }

    #[test]
    fn signed_process_in_temp_is_not_flagged() {
        let session_id = session_id();
        let mut normalized = NormalizedSession::default();
        let mut proc = ProcessInfo::new(session_id, 44, "installer.exe");
        proc.executable_path = Some(r"C:\Users\alice\AppData\Local\Temp\installer.exe".to_string());
        proc.signature_status = SignatureStatus::Signed;
        normalized.processes.insert(44, proc);

        SecurityAnalyzer::analyze(&mut normalized);

        assert!(normalized.processes[&44].security.findings.is_empty());
    }
}
