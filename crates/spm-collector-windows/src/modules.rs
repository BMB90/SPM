use async_trait::async_trait;
use spm_core::{Collector, CollectorCategory, CollectorContext, Event, EventPayload, ModuleInfo, ModuleKind, Platform, SpmResult};
use sysinfo::System;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, MODULEENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32,
};

use crate::util::wide_to_string;

/// Enumerates the DLLs loaded by every running process via a Toolhelp
/// module snapshot. Access-denied on protected system processes is
/// expected and silently skipped (matches Task Manager's behavior when
/// not elevated).
pub struct ModuleCollector;

#[async_trait]
impl Collector for ModuleCollector {
    fn id(&self) -> &'static str {
        "windows.modules"
    }
    fn platform(&self) -> Platform {
        Platform::Windows
    }
    fn category(&self) -> CollectorCategory {
        CollectorCategory::Module
    }
    fn description(&self) -> &'static str {
        "Enumerates loaded DLLs per process via CreateToolhelp32Snapshot(TH32CS_SNAPMODULE)"
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
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut events = Vec::new();
    for pid in sys.processes().keys() {
        let pid_u32 = pid.as_u32();
        for module in modules_for_pid(pid_u32) {
            let mut info = module;
            info.session_id = session_id;
            info.parent_pid = pid_u32;
            events.push(Event::new(session_id, "windows.modules", EventPayload::ModuleLoaded(Box::new(info))));
        }
    }
    events
}

fn modules_for_pid(pid: u32) -> Vec<ModuleInfo> {
    let mut out = Vec::new();
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid) else {
            return out;
        };

        let mut entry = MODULEENTRY32W {
            dwSize: std::mem::size_of::<MODULEENTRY32W>() as u32,
            ..Default::default()
        };

        if Module32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let name = wide_to_string(&entry.szModule);
                let path = wide_to_string(&entry.szExePath);
                out.push(ModuleInfo {
                    id: uuid::Uuid::new_v4(),
                    session_id: uuid::Uuid::nil(),
                    kind: ModuleKind::Dll,
                    name,
                    path: if path.is_empty() { None } else { Some(path) },
                    version: None,
                    signature_status: spm_core::SignatureStatus::Unknown,
                    load_time: None,
                    parent_pid: pid,
                });

                if Module32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
    }
    out
}
