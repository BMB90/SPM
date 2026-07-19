//! Real-time process start/stop tracing via ETW (Event Tracing for
//! Windows), consuming the `Microsoft-Windows-Kernel-Process` provider
//! through `ferrisetw`. This is the collector that lets SPM see processes
//! that started and *exited* before a later snapshot would have caught
//! them — snapshot collectors only see what's still running when they run.
//!
//! Requires an elevated (Administrator) process; `is_available` checks for
//! that up front rather than failing deep inside the ETW session setup.

use std::time::Duration;

use async_trait::async_trait;
use ferrisetw::parser::Parser;
use ferrisetw::provider::Provider;
use ferrisetw::trace::UserTrace;
use ferrisetw::EventRecord;
use spm_core::{
    CollectorCategory, CollectorContext, Event, EventPayload, Platform, ProcessInfo, SpmError, SpmResult,
    StreamingCollector,
};
use tokio::sync::{mpsc, watch};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

const KERNEL_PROCESS_PROVIDER_GUID: &str = "22FB2CD6-0E7B-422B-A0C7-2FAD1FD0E716";
/// How often the tracing thread checks for cooperative cancellation while
/// waiting out the capture window.
const CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(200);

pub struct EtwProcessTraceCollector;

#[async_trait]
impl StreamingCollector for EtwProcessTraceCollector {
    fn id(&self) -> &'static str {
        "windows.etw_process_trace"
    }
    fn platform(&self) -> Platform {
        Platform::Windows
    }
    fn category(&self) -> CollectorCategory {
        CollectorCategory::Process
    }
    fn description(&self) -> &'static str {
        "Real-time process start/stop tracing via ETW's Microsoft-Windows-Kernel-Process provider (requires Administrator)"
    }

    fn is_available(&self) -> SpmResult<()> {
        if is_elevated() {
            Ok(())
        } else {
            Err(SpmError::Unavailable {
                collector: self.id().to_string(),
                reason: "ETW kernel-process tracing requires an elevated (Administrator) process".to_string(),
            })
        }
    }

    async fn stream(&self, ctx: &CollectorContext, tx: mpsc::Sender<Event>) -> SpmResult<()> {
        let session_id = ctx.session.id;
        let capture_window = ctx.capture_window;
        let cancel = ctx.cancel.clone();
        let collector_id = self.id();

        tokio::task::spawn_blocking(move || run_trace(session_id, tx, capture_window, cancel))
            .await
            .map_err(|e| SpmError::Collector { collector: collector_id.to_string(), message: e.to_string() })?;
        Ok(())
    }
}

fn run_trace(session_id: uuid::Uuid, tx: mpsc::Sender<Event>, capture_window: Duration, cancel: watch::Receiver<bool>) {
    let provider = Provider::by_guid(KERNEL_PROCESS_PROVIDER_GUID)
        .add_callback(move |record: &EventRecord, schema_locator: &ferrisetw::schema_locator::SchemaLocator| {
            let Ok(schema) = schema_locator.event_schema(record) else { return };
            let parser = Parser::create(record, &schema);

            let payload = match record.event_id() {
                // ProcessStart
                1 => {
                    let pid: u32 = parser.try_parse("ProcessID").unwrap_or(0);
                    let ppid: u32 = parser.try_parse("ParentProcessID").unwrap_or(0);
                    let image: String = parser.try_parse("ImageName").unwrap_or_default();
                    let mut info = ProcessInfo::new(session_id, pid, image.clone());
                    info.ppid = Some(ppid);
                    info.executable_path = Some(image);
                    info.start_time = Some(chrono::Utc::now());
                    Some(EventPayload::ProcessStarted(Box::new(info)))
                }
                // ProcessStop
                2 => {
                    let pid: u32 = parser.try_parse("ProcessID").unwrap_or(0);
                    Some(EventPayload::ProcessExited { pid, exit_code: None, exit_time: chrono::Utc::now() })
                }
                _ => None,
            };

            if let Some(payload) = payload {
                let event = Event::new(session_id, "windows.etw_process_trace", payload);
                let _ = tx.blocking_send(event);
            }
        })
        .build();

    let Ok(trace) = UserTrace::new().named("spm-etw-process".to_string()).enable(provider).start() else {
        return;
    };

    let deadline = std::time::Instant::now() + capture_window;
    while std::time::Instant::now() < deadline {
        if *cancel.borrow() {
            break;
        }
        std::thread::sleep(CANCEL_POLL_INTERVAL);
    }

    let _ = trace.0.stop();
}

fn is_elevated() -> bool {
    unsafe {
        let mut token = Default::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            size,
            &mut size,
        )
        .is_ok();
        ok && elevation.TokenIsElevated != 0
    }
}
