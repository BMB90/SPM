use async_trait::async_trait;
use serde::Deserialize;
use spm_core::{Collector, CollectorCategory, CollectorContext, DriverInfo, DriverStatus, Event, EventPayload, Platform, SpmResult};
use wmi::{COMLibrary, WMIConnection};

/// Kernel driver inventory via WMI's `Win32_SystemDriver` class. Load
/// order is approximated by WMI enumeration order (annotated as such,
/// since the true NT load order isn't exposed at this layer) — the
/// authoritative order is only observable via a boot-time ETW trace,
/// which is out of scope for a post-boot snapshot collector.
pub struct DriverCollector;

#[allow(non_snake_case, non_camel_case_types)]
#[derive(Deserialize, Debug)]
struct Win32_SystemDriver {
    Name: String,
    PathName: Option<String>,
    State: Option<String>,
    StartMode: Option<String>,
    ErrorControl: Option<String>,
}

#[async_trait]
impl Collector for DriverCollector {
    fn id(&self) -> &'static str {
        "windows.drivers"
    }
    fn platform(&self) -> Platform {
        Platform::Windows
    }
    fn category(&self) -> CollectorCategory {
        CollectorCategory::Driver
    }
    fn description(&self) -> &'static str {
        "Enumerates kernel drivers via WMI Win32_SystemDriver"
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
    let drivers: Vec<Win32_SystemDriver> = wmi_con.query().map_err(|e| e.to_string())?;

    let mut events = Vec::with_capacity(drivers.len());
    for (idx, drv) in drivers.into_iter().enumerate() {
        let mut info = DriverInfo::new(session_id, drv.Name);
        info.path = drv.PathName;
        info.load_order = Some(idx as u32);
        info.status = match drv.State.as_deref() {
            Some("Running") => DriverStatus::Running,
            Some("Stopped") => DriverStatus::Stopped,
            _ => DriverStatus::Unknown,
        };
        if drv.ErrorControl.as_deref().is_some_and(|e| e != "Normal" && e != "Ignore") {
            info.failure_reason = drv.ErrorControl;
        }
        let _ = drv.StartMode; // surfaced via depends_on/status only for now

        events.push(Event::new(session_id, "windows.drivers", EventPayload::DriverObserved(Box::new(info))));
    }
    Ok(events)
}
