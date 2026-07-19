use async_trait::async_trait;
use spm_core::{
    Collector, CollectorCategory, CollectorContext, ConfigAccess, ConfigEntry, ConfigEntryKind, Event, EventPayload,
    Platform, SpmResult,
};
use winreg::enums::*;
use winreg::RegKey;

/// Enumerates the classic Windows autostart locations: the four Run/RunOnce
/// registry keys (HKLM/HKCU x native/Wow6432Node) and the two Startup
/// folders. This is the primary evidence source `spm-analysis`'s
/// startup-source detector uses to explain `RegistryRunKey` /
/// `StartupFolder` classifications.
pub struct StartupRegistryCollector;

const RUN_KEY_PATHS: &[(&str, &str)] = &[
    ("HKLM", r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run"),
    ("HKLM", r"SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce"),
    ("HKLM", r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Run"),
    ("HKLM", r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\RunOnce"),
    ("HKCU", r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run"),
    ("HKCU", r"SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce"),
];

#[async_trait]
impl Collector for StartupRegistryCollector {
    fn id(&self) -> &'static str {
        "windows.startup_registry"
    }
    fn platform(&self) -> Platform {
        Platform::Windows
    }
    fn category(&self) -> CollectorCategory {
        CollectorCategory::Configuration
    }
    fn description(&self) -> &'static str {
        "Enumerates Run/RunOnce registry keys and the Startup folders (all users + current user)"
    }

    async fn collect(&self, ctx: &CollectorContext) -> SpmResult<Vec<Event>> {
        let session_id = ctx.session.id;
        let collector_id = self.id();
        let events = tokio::task::spawn_blocking(move || collect_sync(session_id))
            .await
            .map_err(|e| spm_core::SpmError::Collector { collector: collector_id.to_string(), message: e.to_string() })?;
        Ok(events)
    }
}

fn collect_sync(session_id: uuid::Uuid) -> Vec<Event> {
    let mut events = Vec::new();

    for (hive_name, subkey) in RUN_KEY_PATHS {
        let hive = match *hive_name {
            "HKLM" => RegKey::predef(HKEY_LOCAL_MACHINE),
            "HKCU" => RegKey::predef(HKEY_CURRENT_USER),
            _ => continue,
        };
        let Ok(key) = hive.open_subkey(subkey) else { continue };
        for item in key.enum_values() {
            let Ok((name, value)) = item else { continue };
            let entry = ConfigEntry {
                id: uuid::Uuid::new_v4(),
                session_id,
                kind: ConfigEntryKind::RegistryValue,
                location: format!(r"{hive_name}\{subkey}"),
                name: Some(name),
                value: Some(value.to_string()),
                access: ConfigAccess::Read,
                pid: None,
                related_startup_items: Vec::new(),
            };
            events.push(Event::new(
                session_id,
                "windows.startup_registry",
                EventPayload::ConfigEntryObserved(Box::new(entry)),
            ));
        }
    }

    for folder in startup_folders() {
        let Ok(read_dir) = std::fs::read_dir(&folder) else { continue };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let config_entry = ConfigEntry {
                id: uuid::Uuid::new_v4(),
                session_id,
                kind: ConfigEntryKind::GenericConfigFile,
                location: format!("StartupFolder:{}", folder.display()),
                name: path.file_name().map(|n| n.to_string_lossy().to_string()),
                value: Some(path.to_string_lossy().to_string()),
                access: ConfigAccess::Read,
                pid: None,
                related_startup_items: Vec::new(),
            };
            events.push(Event::new(
                session_id,
                "windows.startup_registry",
                EventPayload::ConfigEntryObserved(Box::new(config_entry)),
            ));
        }
    }

    events
}

fn startup_folders() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    if let Ok(appdata) = std::env::var("APPDATA") {
        paths.push(std::path::PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs\Startup"));
    }
    if let Ok(programdata) = std::env::var("ProgramData") {
        paths.push(std::path::PathBuf::from(programdata).join(r"Microsoft\Windows\Start Menu\Programs\StartUp"));
    }
    paths
}
