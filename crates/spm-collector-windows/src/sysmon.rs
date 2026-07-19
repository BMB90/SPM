//! Optional Sysmon collector. Sysmon (Microsoft Sysinternals) isn't part
//! of a stock Windows install; when present it logs far richer process
//! creation detail (full command line, hashes, parent image) to the
//! `Microsoft-Windows-Sysmon/Operational` channel than the base System
//! log does. `is_available` checks for the Sysmon service before this
//! collector does anything, so a stock machine simply skips it.

use async_trait::async_trait;
use spm_core::{
    Collector, CollectorCategory, CollectorContext, Event, EventPayload, Platform, SpmError, SpmResult,
};
use windows::core::PCWSTR;
use windows::Win32::System::EventLog::{EvtClose, EvtNext, EvtQuery, EvtQueryChannelPath, EvtQueryReverseDirection, EVT_HANDLE};
use winreg::enums::*;
use winreg::RegKey;

use crate::eventlog::render_event;
use crate::util::to_wide;

const SYSMON_CHANNEL: &str = "Microsoft-Windows-Sysmon/Operational";
const MAX_EVENTS: u32 = 200;

pub struct SysmonCollector;

#[async_trait]
impl Collector for SysmonCollector {
    fn id(&self) -> &'static str {
        "windows.sysmon"
    }
    fn platform(&self) -> Platform {
        Platform::Windows
    }
    fn category(&self) -> CollectorCategory {
        CollectorCategory::Process
    }
    fn description(&self) -> &'static str {
        "Reads process-creation events from Sysmon's event channel, when Sysmon is installed"
    }

    fn is_available(&self) -> SpmResult<()> {
        if sysmon_service_present() {
            Ok(())
        } else {
            Err(SpmError::Unavailable {
                collector: self.id().to_string(),
                reason: "Sysmon is not installed on this system".to_string(),
            })
        }
    }

    async fn collect(&self, ctx: &CollectorContext) -> SpmResult<Vec<Event>> {
        let session_id = ctx.session.id;
        let collector_id = self.id();
        let events = tokio::task::spawn_blocking(move || collect_sync(session_id))
            .await
            .map_err(|e| SpmError::Collector { collector: collector_id.to_string(), message: e.to_string() })?
            .map_err(|e| SpmError::Collector { collector: collector_id.to_string(), message: e })?;
        Ok(events)
    }
}

fn sysmon_service_present() -> bool {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    hklm.open_subkey(r"SYSTEM\CurrentControlSet\Services\Sysmon").is_ok()
        || hklm.open_subkey(r"SYSTEM\CurrentControlSet\Services\Sysmon64").is_ok()
}

fn collect_sync(session_id: uuid::Uuid) -> Result<Vec<Event>, String> {
    let channel = to_wide(SYSMON_CHANNEL);
    let query = to_wide("*[System[(EventID=1)]]"); // ProcessCreate only

    unsafe {
        let result_set = EvtQuery(
            None,
            PCWSTR::from_raw(channel.as_ptr()),
            PCWSTR::from_raw(query.as_ptr()),
            (EvtQueryChannelPath.0 | EvtQueryReverseDirection.0) as u32,
        )
        .map_err(|e| format!("EvtQuery(Sysmon) failed: {e}"))?;

        let mut events = Vec::new();
        let mut handles = vec![0isize; 32];

        loop {
            let mut returned = 0u32;
            if EvtNext(result_set, &mut handles, 1000, 0, &mut returned).is_err() || returned == 0 {
                break;
            }
            for &raw_handle in &handles[..returned as usize] {
                let handle = EVT_HANDLE(raw_handle);
                if let Some(parsed) = render_event(handle) {
                    events.push(Event::new(
                        session_id,
                        "windows.sysmon",
                        EventPayload::Raw {
                            source: "windows.sysmon".to_string(),
                            kind: "sysmon_process_create".to_string(),
                            payload: serde_json::json!({
                                "event_id": parsed.event_id,
                                "provider": parsed.provider,
                                "time_created": parsed.time_created,
                            }),
                        },
                    ));
                }
                let _ = EvtClose(handle);
            }
            if events.len() as u32 >= MAX_EVENTS {
                break;
            }
        }

        let _ = EvtClose(result_set);
        Ok(events)
    }
}
