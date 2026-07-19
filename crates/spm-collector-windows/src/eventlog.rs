//! Windows Event Log collector using the modern `Evt*` (wevtapi) query
//! API. Pulls recent System-channel events relevant to boot/service
//! lifecycle (Service Control Manager 7036 "service entered the X state",
//! Kernel-General 12/13 boot/shutdown markers, EventLog 6005/6009/6013)
//! and surfaces them as `BootStageReached`/`Raw` events for the timeline.

use async_trait::async_trait;
use quick_xml::events::Event as XmlEvent;
use quick_xml::reader::Reader;
use spm_core::{Collector, CollectorCategory, CollectorContext, Event, EventPayload, Platform, SpmResult};
use windows::core::PCWSTR;
use windows::Win32::System::EventLog::{
    EvtClose, EvtNext, EvtQuery, EvtRender, EvtQueryChannelPath, EvtQueryReverseDirection, EvtRenderEventXml,
    EVT_HANDLE,
};

use crate::util::to_wide;

const MAX_EVENTS: u32 = 300;

/// Boot/service-lifecycle-relevant System-channel event IDs we bother
/// parsing; everything else is skipped to keep the capture window fast.
const RELEVANT_EVENT_IDS: &[u32] = &[12, 13, 6005, 6006, 6008, 6009, 6013, 7036, 7040, 7000, 7001, 7009, 7011, 7031, 7034];

pub struct EventLogCollector;

#[async_trait]
impl Collector for EventLogCollector {
    fn id(&self) -> &'static str {
        "windows.event_log"
    }
    fn platform(&self) -> Platform {
        Platform::Windows
    }
    fn category(&self) -> CollectorCategory {
        CollectorCategory::BootStage
    }
    fn description(&self) -> &'static str {
        "Queries the System event log for boot/service-lifecycle events (SCM 7036, Kernel-General 12/13, EventLog 6005/6009/6013)"
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
    let channel = to_wide("System");
    let query = to_wide("*");

    unsafe {
        let result_set = EvtQuery(
            None,
            PCWSTR::from_raw(channel.as_ptr()),
            PCWSTR::from_raw(query.as_ptr()),
            (EvtQueryChannelPath.0 | EvtQueryReverseDirection.0) as u32,
        )
        .map_err(|e| format!("EvtQuery failed: {e}"))?;

        let mut events = Vec::new();
        let mut handles = vec![0isize; 32];

        'outer: loop {
            let mut returned = 0u32;
            let next_result = EvtNext(result_set, &mut handles, 1000, 0, &mut returned);
            if next_result.is_err() || returned == 0 {
                break;
            }

            for &raw_handle in &handles[..returned as usize] {
                let handle = EVT_HANDLE(raw_handle);
                if let Some(parsed) = render_event(handle) {
                    if RELEVANT_EVENT_IDS.contains(&parsed.event_id) {
                        events.push(Event::new(
                            session_id,
                            "windows.event_log",
                            EventPayload::Raw {
                                source: "windows.event_log".to_string(),
                                kind: "system_event".to_string(),
                                payload: serde_json::json!({
                                    "event_id": parsed.event_id,
                                    "provider": parsed.provider,
                                    "time_created": parsed.time_created,
                                }),
                            },
                        ));
                    }
                }
                let _ = EvtClose(handle);
            }

            if events.len() as u32 >= MAX_EVENTS {
                break 'outer;
            }
        }

        let _ = EvtClose(result_set);
        Ok(events)
    }
}

pub(crate) struct ParsedEvent {
    pub event_id: u32,
    pub provider: String,
    pub time_created: String,
}

pub(crate) fn render_event(handle: EVT_HANDLE) -> Option<ParsedEvent> {
    unsafe {
        let mut buffer_used = 0u32;
        let mut property_count = 0u32;
        // First call with no buffer to learn the required size.
        let _ = EvtRender(None, handle, EvtRenderEventXml.0 as u32, 0, None, &mut buffer_used, &mut property_count);
        if buffer_used == 0 {
            return None;
        }
        let mut buffer = vec![0u8; buffer_used as usize];
        EvtRender(
            None,
            handle,
            EvtRenderEventXml.0 as u32,
            buffer_used,
            Some(buffer.as_mut_ptr() as *mut _),
            &mut buffer_used,
            &mut property_count,
        )
        .ok()?;

        // The buffer is UTF-16.
        let (_, aligned, _) = buffer.align_to::<u16>();
        let xml = String::from_utf16_lossy(aligned);
        let xml = xml.trim_end_matches('\0');
        parse_event_xml(xml)
    }
}

pub(crate) fn parse_event_xml(xml: &str) -> Option<ParsedEvent> {
    let mut reader = Reader::from_str(xml);
    let mut event_id = None;
    let mut provider = String::new();
    let mut time_created = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Empty(e)) | Ok(XmlEvent::Start(e)) => {
                let name = e.name();
                let local = String::from_utf8_lossy(name.as_ref()).to_string();
                match local.as_str() {
                    "Provider" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"Name" {
                                provider = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                    "TimeCreated" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"SystemTime" {
                                time_created = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(XmlEvent::Text(t)) => {
                if event_id.is_none() {
                    if let Ok(text) = t.unescape() {
                        if let Ok(id) = text.trim().parse::<u32>() {
                            event_id = Some(id);
                        }
                    }
                }
            }
            Ok(XmlEvent::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    event_id.map(|id| ParsedEvent { event_id: id, provider, time_created })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_EVENT_XML: &str = r#"<Event xmlns="http://schemas.microsoft.com/win/2004/08/events/event">
  <System>
    <Provider Name="Service Control Manager" Guid="{555908d1-a6d7-4695-8e1e-26931d2012f4}" EventSourceName="Service Control Manager"/>
    <EventID Qualifiers="16384">7036</EventID>
    <Version>0</Version>
    <Level>4</Level>
    <Task>0</Task>
    <Keywords>0x8080000000000000</Keywords>
    <TimeCreated SystemTime="2026-07-20T02:57:54.1234567Z"/>
    <EventRecordID>12345</EventRecordID>
    <Channel>System</Channel>
    <Computer>DESKTOP-TEST</Computer>
    <Security/>
  </System>
  <EventData>
    <Data Name="param1">Windows Update</Data>
    <Data Name="param2">running</Data>
  </EventData>
</Event>"#;

    #[test]
    fn parses_event_id_provider_and_time() {
        let parsed = parse_event_xml(SAMPLE_EVENT_XML).expect("parses");
        assert_eq!(parsed.event_id, 7036);
        assert_eq!(parsed.provider, "Service Control Manager");
        assert_eq!(parsed.time_created, "2026-07-20T02:57:54.1234567Z");
    }

    #[test]
    fn returns_none_for_xml_without_event_id() {
        let xml = r#"<Event><System><Provider Name="X"/></System></Event>"#;
        assert!(parse_event_xml(xml).is_none());
    }

    #[test]
    fn relevant_event_ids_cover_scm_and_kernel_markers() {
        assert!(RELEVANT_EVENT_IDS.contains(&7036));
        assert!(RELEVANT_EVENT_IDS.contains(&12));
        assert!(RELEVANT_EVENT_IDS.contains(&6005));
    }
}
