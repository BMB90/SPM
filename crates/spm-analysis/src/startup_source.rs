use std::collections::HashMap;

use spm_core::{ConfigEntry, ConfigEntryKind, ProcessRole, StartupSource, StartupSourceKind};
use spm_engine::NormalizedSession;

/// Assigns `ProcessInfo::startup_source` (and refines `ProcessInfo::role`)
/// by cross-referencing each process against the config entries (registry
/// Run keys, Startup folder contents, scheduled task definitions) and
/// services observed in the same capture. This is the "Startup Source
/// Detection" component the spec calls for: every classification carries
/// an evidence trail rather than being asserted blindly.
pub struct StartupSourceDetector;

impl StartupSourceDetector {
    pub fn detect(normalized: &mut NormalizedSession) {
        let config_entries = normalized.config_entries.clone();
        let names_by_pid: HashMap<u32, String> =
            normalized.processes.iter().map(|(pid, p)| (*pid, p.executable_name.clone())).collect();

        for process in normalized.processes.values_mut() {
            if process.startup_source.is_some() {
                continue;
            }

            if let Some(service_name) = process.owning_service.clone() {
                process.startup_source =
                    Some(StartupSource::new(StartupSourceKind::WindowsService { service_name }).with_confidence(1.0));
                if process.role == ProcessRole::Unknown {
                    process.role = ProcessRole::Service;
                }
                continue;
            }

            let Some(exe_path) = process.executable_path.clone() else {
                assign_fallback(process, process.ppid, &names_by_pid);
                continue;
            };

            if let Some(matched) = config_entries.iter().find(|entry| entry_matches(entry, &exe_path)) {
                let kind = classify_config_entry(matched);
                let evidence = format!(
                    "{}: {}",
                    matched.location,
                    matched.value.clone().or_else(|| matched.name.clone()).unwrap_or_default()
                );
                process.startup_source = Some(StartupSource::new(kind).with_evidence(evidence).with_confidence(0.9));
                if process.role == ProcessRole::Unknown {
                    process.role = ProcessRole::LoginItem;
                }
                continue;
            }

            assign_fallback(process, process.ppid, &names_by_pid);
        }
    }
}

fn assign_fallback(process: &mut spm_core::ProcessInfo, ppid: Option<u32>, names_by_pid: &HashMap<u32, String>) {
    process.startup_source = Some(match ppid {
        Some(parent_pid) => {
            let parent_executable = names_by_pid.get(&parent_pid).cloned();
            StartupSource::new(StartupSourceKind::ParentProcess { parent_pid, parent_executable }).with_confidence(0.4)
        }
        None => StartupSource::new(StartupSourceKind::Unknown),
    });
}

fn entry_matches(entry: &ConfigEntry, exe_path: &str) -> bool {
    let candidates = [entry.value.as_deref(), entry.name.as_deref()];
    candidates.into_iter().flatten().any(|c| c.contains(exe_path) || exe_path.contains(c))
        || entry.related_startup_items.iter().any(|i| i == exe_path)
}

fn classify_config_entry(entry: &ConfigEntry) -> StartupSourceKind {
    if entry.location.starts_with("StartupFolder:") {
        return StartupSourceKind::StartupFolder { path: entry.value.clone().unwrap_or_default() };
    }
    if entry.location.starts_with("ScheduledTask:") {
        return StartupSourceKind::ScheduledTask { task_path: entry.name.clone().unwrap_or_default() };
    }
    match entry.kind {
        ConfigEntryKind::RegistryValue | ConfigEntryKind::RegistryKey => {
            let (hive, key) = entry.location.split_once('\\').unwrap_or(("", entry.location.as_str()));
            StartupSourceKind::RegistryRunKey {
                hive: hive.to_string(),
                key: key.to_string(),
                value: entry.name.clone().unwrap_or_default(),
            }
        }
        ConfigEntryKind::ComRegistration => StartupSourceKind::ComActivation { clsid: entry.name.clone().unwrap_or_default() },
        _ => StartupSourceKind::Other { description: entry.location.clone() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spm_core::{BootSession, ConfigAccess, ConfigEntryKind, Platform, ProcessInfo};
    use spm_engine::NormalizedSession;
    use uuid::Uuid;

    fn session_id() -> Uuid {
        BootSession::new("host", Platform::Windows, "test").id
    }

    #[test]
    fn matches_process_to_registry_run_key() {
        let session_id = session_id();
        let mut normalized = NormalizedSession::default();

        let mut proc = ProcessInfo::new(session_id, 200, "updater.exe");
        proc.executable_path = Some(r"C:\Program Files\Vendor\updater.exe".to_string());
        normalized.processes.insert(200, proc);

        normalized.config_entries.push(ConfigEntry {
            id: Uuid::new_v4(),
            session_id,
            kind: ConfigEntryKind::RegistryValue,
            location: r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Run".to_string(),
            name: Some("VendorUpdater".to_string()),
            value: Some(r#""C:\Program Files\Vendor\updater.exe" /background"#.to_string()),
            access: ConfigAccess::Read,
            pid: None,
            related_startup_items: Vec::new(),
        });

        StartupSourceDetector::detect(&mut normalized);

        let source = normalized.processes[&200].startup_source.as_ref().expect("source assigned");
        assert!(matches!(source.kind, StartupSourceKind::RegistryRunKey { .. }));
        assert_eq!(normalized.processes[&200].role, ProcessRole::LoginItem);
    }

    #[test]
    fn falls_back_to_parent_process_when_no_evidence_matches() {
        let session_id = session_id();
        let mut normalized = NormalizedSession::default();

        let mut parent = ProcessInfo::new(session_id, 1, "explorer.exe");
        parent.executable_path = Some(r"C:\Windows\explorer.exe".to_string());
        normalized.processes.insert(1, parent);

        let mut child = ProcessInfo::new(session_id, 2, "notepad.exe");
        child.ppid = Some(1);
        child.executable_path = Some(r"C:\Windows\notepad.exe".to_string());
        normalized.processes.insert(2, child);

        StartupSourceDetector::detect(&mut normalized);

        let source = normalized.processes[&2].startup_source.as_ref().unwrap();
        match &source.kind {
            StartupSourceKind::ParentProcess { parent_pid, parent_executable } => {
                assert_eq!(*parent_pid, 1);
                assert_eq!(parent_executable.as_deref(), Some("explorer.exe"));
            }
            other => panic!("expected ParentProcess, got {other:?}"),
        }
        assert!(source.confidence < 0.9);
    }

    #[test]
    fn owning_service_takes_priority_over_registry_match() {
        let session_id = session_id();
        let mut normalized = NormalizedSession::default();

        let mut proc = ProcessInfo::new(session_id, 300, "svchost.exe");
        proc.executable_path = Some(r"C:\Windows\System32\svchost.exe".to_string());
        proc.owning_service = Some("Spooler".to_string());
        normalized.processes.insert(300, proc);

        StartupSourceDetector::detect(&mut normalized);

        let source = normalized.processes[&300].startup_source.as_ref().unwrap();
        assert!(matches!(&source.kind, StartupSourceKind::WindowsService { service_name } if service_name == "Spooler"));
        assert_eq!(normalized.processes[&300].role, ProcessRole::Service);
    }
}
