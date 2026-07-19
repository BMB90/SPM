use async_trait::async_trait;
use serde::Deserialize;
use spm_core::{
    Collector, CollectorCategory, CollectorContext, Event, EventPayload, Platform, ServiceInfo, ServiceStartType,
    ServiceState, SpmResult,
};
use winreg::enums::*;
use winreg::RegKey;
use wmi::{COMLibrary, WMIConnection};

/// Windows Service Control Manager inventory, sourced from WMI's
/// `Win32_Service` class and cross-referenced against the service's
/// registry key (`HKLM\SYSTEM\CurrentControlSet\Services\<name>`) for
/// dependency lists and delayed-autostart status that WMI doesn't expose.
pub struct ServiceCollector;

#[allow(non_snake_case, non_camel_case_types)]
#[derive(Deserialize, Debug)]
struct Win32_Service {
    Name: String,
    DisplayName: Option<String>,
    PathName: Option<String>,
    Description: Option<String>,
    State: Option<String>,
    StartMode: Option<String>,
    StartName: Option<String>,
    ProcessId: Option<u32>,
}

#[async_trait]
impl Collector for ServiceCollector {
    fn id(&self) -> &'static str {
        "windows.services"
    }
    fn platform(&self) -> Platform {
        Platform::Windows
    }
    fn category(&self) -> CollectorCategory {
        CollectorCategory::Service
    }
    fn description(&self) -> &'static str {
        "Enumerates Windows services via WMI Win32_Service, enriched with registry dependency data"
    }

    async fn collect(&self, ctx: &CollectorContext) -> SpmResult<Vec<Event>> {
        let session_id = ctx.session.id;
        let collector_id = self.id();
        let events = tokio::task::spawn_blocking(move || collect_sync(session_id))
            .await
            .map_err(|e| spm_core::SpmError::Collector { collector: collector_id.to_string(), message: e.to_string() })?
            .map_err(|e| spm_core::SpmError::Collector { collector: collector_id.to_string(), message: e })?;
        Ok(events)
    }
}

fn collect_sync(session_id: uuid::Uuid) -> Result<Vec<Event>, String> {
    let com_con = COMLibrary::new().map_err(|e| e.to_string())?;
    let wmi_con = WMIConnection::new(com_con).map_err(|e| e.to_string())?;
    let services: Vec<Win32_Service> = wmi_con.query().map_err(|e| e.to_string())?;

    let mut events = Vec::with_capacity(services.len());
    for svc in services {
        let (depends_on, delayed) = registry_service_details(&svc.Name);

        let mut info = ServiceInfo::new(session_id, svc.Name.clone());
        info.display_name = svc.DisplayName;
        info.description = svc.Description;
        info.binary_path = svc.PathName;
        info.config_path = Some(format!(r"HKLM\SYSTEM\CurrentControlSet\Services\{}", svc.Name));
        info.state = parse_state(svc.State.as_deref());
        info.start_type = parse_start_type(svc.StartMode.as_deref(), delayed);
        info.owner = svc.StartName;
        info.pid = svc.ProcessId.filter(|&p| p != 0);
        info.depends_on = depends_on;

        events.push(Event::new(session_id, "windows.services", EventPayload::ServiceObserved(Box::new(info))));
    }
    Ok(events)
}

fn registry_service_details(name: &str) -> (Vec<String>, bool) {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let Ok(key) = hklm.open_subkey(format!(r"SYSTEM\CurrentControlSet\Services\{name}")) else {
        return (Vec::new(), false);
    };
    let depends_on: Vec<String> = key.get_value::<Vec<String>, _>("DependOnService").unwrap_or_default();
    let delayed: u32 = key.get_value("DelayedAutostart").unwrap_or(0);
    (depends_on, delayed != 0)
}

fn parse_state(state: Option<&str>) -> ServiceState {
    match state {
        Some("Running") => ServiceState::Running,
        Some("Stopped") => ServiceState::Stopped,
        Some("Start Pending") => ServiceState::StartPending,
        Some("Stop Pending") => ServiceState::StopPending,
        Some("Paused") => ServiceState::Paused,
        _ => ServiceState::Unknown,
    }
}

fn parse_start_type(mode: Option<&str>, delayed: bool) -> ServiceStartType {
    match mode {
        Some("Boot") => ServiceStartType::Boot,
        Some("System") => ServiceStartType::System,
        Some("Auto") if delayed => ServiceStartType::AutomaticDelayedStart,
        Some("Auto") => ServiceStartType::Automatic,
        Some("Manual") => ServiceStartType::Manual,
        Some("Disabled") => ServiceStartType::Disabled,
        _ => ServiceStartType::Unknown,
    }
}
