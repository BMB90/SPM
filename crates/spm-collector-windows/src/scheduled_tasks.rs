//! Scheduled Task collector.
//!
//! Reads Task Scheduler's on-disk task definitions
//! (`%WINDIR%\System32\Tasks\**`, one XML file per task) rather than going
//! through the `ITaskService` COM automation interface. This avoids a
//! large surface of IDispatch/VARIANT marshalling for the same data the
//! XML already contains, at the cost of needing read access to the Tasks
//! directory (present for admins; some protected tasks under `Microsoft\
//! Windows\...` may be unreadable without elevation and are skipped).

use async_trait::async_trait;
use quick_xml::events::Event as XmlEvent;
use quick_xml::reader::Reader;
use spm_core::{
    Collector, CollectorCategory, CollectorContext, ConfigAccess, ConfigEntry, ConfigEntryKind, Event, EventPayload,
    Platform, SpmResult,
};

pub struct ScheduledTaskCollector;

#[async_trait]
impl Collector for ScheduledTaskCollector {
    fn id(&self) -> &'static str {
        "windows.scheduled_tasks"
    }
    fn platform(&self) -> Platform {
        Platform::Windows
    }
    fn category(&self) -> CollectorCategory {
        CollectorCategory::Configuration
    }
    fn description(&self) -> &'static str {
        "Enumerates Task Scheduler task definitions from %WINDIR%\\System32\\Tasks"
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
    let root = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".to_string());
    let tasks_dir = std::path::PathBuf::from(root).join(r"System32\Tasks");

    let mut events = Vec::new();
    walk(&tasks_dir, &tasks_dir, session_id, &mut events);
    events
}

fn walk(root: &std::path::Path, dir: &std::path::Path, session_id: uuid::Uuid, events: &mut Vec<Event>) {
    let Ok(read_dir) = std::fs::read_dir(dir) else { return };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, session_id, events);
            continue;
        }
        let Ok(xml) = std::fs::read_to_string(&path) else { continue };
        let Some(parsed) = parse_task_xml(&xml) else { continue };

        let task_path = path
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.to_string_lossy().to_string());

        let value = serde_json::json!({
            "command": parsed.command,
            "arguments": parsed.arguments,
            "triggers": parsed.triggers,
            "enabled": parsed.enabled,
        })
        .to_string();

        let entry = ConfigEntry {
            id: uuid::Uuid::new_v4(),
            session_id,
            kind: ConfigEntryKind::GenericConfigFile,
            location: format!("ScheduledTask:{task_path}"),
            name: Some(task_path),
            value: Some(value),
            access: ConfigAccess::Read,
            pid: None,
            related_startup_items: parsed.command.clone().into_iter().collect(),
        };
        events.push(Event::new(session_id, "windows.scheduled_tasks", EventPayload::ConfigEntryObserved(Box::new(entry))));
    }
}

#[derive(Default)]
struct ParsedTask {
    command: Option<String>,
    arguments: Option<String>,
    triggers: Vec<String>,
    enabled: Option<bool>,
}

fn parse_task_xml(xml: &str) -> Option<ParsedTask> {
    let mut reader = Reader::from_str(xml);
    let mut task = ParsedTask::default();
    let mut buf = Vec::new();
    let mut current_tag = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(e)) | Ok(XmlEvent::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local = name.rsplit(':').next().unwrap_or(&name).to_string();
                current_tag = local.clone();
                if local.ends_with("Trigger") {
                    task.triggers.push(local);
                }
            }
            Ok(XmlEvent::Text(t)) => {
                if let Ok(text) = t.unescape() {
                    let text = text.trim().to_string();
                    if text.is_empty() {
                        continue;
                    }
                    match current_tag.as_str() {
                        "Command" => task.command = Some(text),
                        "Arguments" => task.arguments = Some(text),
                        "Enabled" => task.enabled = text.parse::<bool>().ok(),
                        _ => {}
                    }
                }
            }
            Ok(XmlEvent::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    if task.command.is_none() && task.triggers.is_empty() {
        return None;
    }
    Some(task)
}
