use std::collections::HashMap;

use spm_core::{
    ConfigEntry, DriverInfo, Event, EventPayload, FileActivity, ModuleInfo, NetworkActivity,
    ProcessInfo, ServiceInfo, ServiceState,
};
use tracing::warn;
use uuid::Uuid;

/// A deduplicated, merged view of every entity observed by every collector
/// during one capture session. Collectors may report the same process
/// multiple times (e.g. a snapshot collector and a streaming ETW collector
/// both see PID 4021) — `EventProcessor` merges those into one record per
/// PID rather than letting duplicates leak into storage/UI.
#[derive(Debug, Clone, Default)]
pub struct NormalizedSession {
    pub processes: HashMap<u32, ProcessInfo>,
    pub services: HashMap<String, ServiceInfo>,
    pub drivers: HashMap<String, DriverInfo>,
    pub modules: Vec<ModuleInfo>,
    pub file_activity: Vec<FileActivity>,
    pub network_activity: Vec<NetworkActivity>,
    pub config_entries: Vec<ConfigEntry>,
    pub boot_stage_events: Vec<(spm_core::BootStage, chrono::DateTime<chrono::Utc>, Option<String>)>,
    pub unrecognized: Vec<(String, String, serde_json::Value)>,
}

/// Consumes raw collector `Event`s and produces one merged `NormalizedSession`.
pub struct EventProcessor {
    session_id: Uuid,
}

impl EventProcessor {
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }

    pub fn process(&self, events: Vec<Event>) -> NormalizedSession {
        let mut out = NormalizedSession::default();

        for event in events {
            if event.session_id != self.session_id {
                warn!(
                    expected = %self.session_id,
                    actual = %event.session_id,
                    collector = %event.collector_id,
                    "dropping event from foreign session"
                );
                continue;
            }

            match event.payload {
                EventPayload::ProcessStarted(p) | EventPayload::ProcessUpdated(p) => {
                    merge_process(&mut out.processes, *p);
                }
                EventPayload::ProcessExited { pid, exit_code, exit_time } => {
                    if let Some(existing) = out.processes.get_mut(&pid) {
                        existing.exit_time = Some(exit_time);
                        existing.exit_code = exit_code;
                    }
                }
                EventPayload::ServiceObserved(s) => {
                    out.services.insert(s.name.clone(), *s);
                }
                EventPayload::ServiceStateChanged { name, state, timestamp } => {
                    if let Some(existing) = out.services.get_mut(&name) {
                        existing.state = state;
                        if state == ServiceState::Running && existing.start_time.is_none() {
                            existing.start_time = Some(timestamp);
                        }
                    }
                }
                EventPayload::DriverObserved(d) => {
                    out.drivers.insert(d.name.clone(), *d);
                }
                EventPayload::ModuleLoaded(m) => out.modules.push(*m),
                EventPayload::FileActivityObserved(f) => out.file_activity.push(*f),
                EventPayload::NetworkActivityObserved(n) => out.network_activity.push(*n),
                EventPayload::ConfigEntryObserved(c) => out.config_entries.push(*c),
                EventPayload::BootStageReached { stage, timestamp, detail } => {
                    out.boot_stage_events.push((stage, timestamp, detail));
                }
                EventPayload::Raw { source, kind, payload } => {
                    out.unrecognized.push((source, kind, payload));
                }
            }
        }

        out.boot_stage_events.sort_by_key(|(_, ts, _)| *ts);
        out
    }
}

/// Merge a newly-observed `ProcessInfo` into the map, preferring populated
/// fields from the incoming record but never clobbering an already-known
/// field with `None`.
fn merge_process(map: &mut HashMap<u32, ProcessInfo>, incoming: ProcessInfo) {
    match map.get_mut(&incoming.pid) {
        None => {
            map.insert(incoming.pid, incoming);
        }
        Some(existing) => {
            macro_rules! prefer_incoming {
                ($field:ident) => {
                    if incoming.$field.is_some() {
                        existing.$field = incoming.$field.clone();
                    }
                };
            }
            prefer_incoming!(ppid);
            prefer_incoming!(executable_path);
            prefer_incoming!(working_directory);
            prefer_incoming!(command_line);
            prefer_incoming!(start_time);
            prefer_incoming!(exit_time);
            prefer_incoming!(exit_code);
            prefer_incoming!(user);
            prefer_incoming!(group);
            prefer_incoming!(thread_count);
            prefer_incoming!(handle_count);
            prefer_incoming!(sha256);
            prefer_incoming!(signer);
            prefer_incoming!(owning_service);
            prefer_incoming!(startup_source);

            if !incoming.arguments.is_empty() {
                existing.arguments = incoming.arguments;
            }
            if !incoming.environment.is_empty() {
                existing.environment = incoming.environment;
            }
            if incoming.signature_status != spm_core::SignatureStatus::Unknown {
                existing.signature_status = incoming.signature_status;
            }
            if incoming.role != spm_core::ProcessRole::Unknown {
                existing.role = incoming.role;
            }
            if incoming.metadata.version.is_some() {
                existing.metadata = incoming.metadata;
            }
            // Performance metrics: keep the peak of any peak-style field.
            let perf = &mut existing.performance;
            let inc = &incoming.performance;
            perf.cpu_percent_peak = max_opt(perf.cpu_percent_peak, inc.cpu_percent_peak);
            perf.memory_bytes_peak = max_opt(perf.memory_bytes_peak, inc.memory_bytes_peak);
            perf.thread_count_peak = max_opt(perf.thread_count_peak, inc.thread_count_peak);
            if inc.cpu_time_ms.is_some() {
                perf.cpu_time_ms = inc.cpu_time_ms;
            }
            if inc.memory_bytes_current.is_some() {
                perf.memory_bytes_current = inc.memory_bytes_current;
            }
        }
    }
}

fn max_opt<T: PartialOrd + Copy>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(x), Some(y)) => Some(if x > y { x } else { y }),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}
