use std::collections::HashSet;

use chrono::{DateTime, Utc};
use spm_core::{BootSession, BootStage, ProcessInfo, TimelineEntry};
use uuid::Uuid;

use crate::dependency::{process_node_id, service_node_id};
use crate::normalize::NormalizedSession;

/// Entries within this many milliseconds of each other, within the same
/// stage, are considered to have started "in parallel" for the UI's
/// parallel-execution visualization.
const PARALLEL_WINDOW_MS: i64 = 150;

pub struct TimelineBuilder;

impl TimelineBuilder {
    /// Build the sorted, offset-computed timeline. `critical_path` should
    /// be the node-id set from `DependencyGraphBuilder::critical_path`.
    pub fn build(
        session: &BootSession,
        normalized: &NormalizedSession,
        critical_path: &HashSet<String>,
    ) -> Vec<TimelineEntry> {
        let reference = session.boot_time.unwrap_or(session.capture_started_at);
        let mut entries = Vec::new();

        for (stage, timestamp, detail) in &normalized.boot_stage_events {
            entries.push(raw_entry(session.id, *stage, detail.clone().unwrap_or_default(), *timestamp, reference, "boot_stage", String::new()));
        }

        for process in normalized.processes.values() {
            let Some(start) = process.start_time else { continue };
            let stage = infer_process_stage(process);
            let node_id = process_node_id(process.pid);
            let mut entry = raw_entry(
                session.id,
                stage,
                process.executable_name.clone(),
                start,
                reference,
                "process",
                process.pid.to_string(),
            );
            entry.duration_ms = process
                .exit_time
                .map(|exit| (exit - start).num_milliseconds().max(0) as u64);
            entry.on_critical_path = critical_path.contains(&node_id);
            entries.push(entry);
        }

        for service in normalized.services.values() {
            let Some(start) = service.start_time else { continue };
            let node_id = service_node_id(&service.name);
            let mut entry = raw_entry(
                session.id,
                BootStage::ServiceStartup,
                service.name.clone(),
                start,
                reference,
                "service",
                service.name.clone(),
            );
            entry.duration_ms = service
                .end_time
                .map(|end| (end - start).num_milliseconds().max(0) as u64);
            entry.on_critical_path = critical_path.contains(&node_id);
            entries.push(entry);
        }

        for driver in normalized.drivers.values() {
            let Some(load_time) = driver.load_time else { continue };
            entries.push(raw_entry(
                session.id,
                BootStage::DriverInit,
                driver.name.clone(),
                load_time,
                reference,
                "driver",
                driver.name.clone(),
            ));
        }

        entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        assign_parallel_groups(&mut entries);
        entries
    }
}

fn infer_process_stage(process: &ProcessInfo) -> BootStage {
    use spm_core::ProcessRole::*;
    match process.role {
        KernelProcess => BootStage::Kernel,
        Service | Daemon => BootStage::ServiceStartup,
        ScheduledTask => BootStage::ScheduledTasks,
        LoginItem => BootStage::StartupApplications,
        UserApplication => BootStage::StartupApplications,
        System | Unknown => BootStage::Unknown,
    }
}

fn raw_entry(
    session_id: Uuid,
    stage: BootStage,
    label: String,
    timestamp: DateTime<Utc>,
    reference: DateTime<Utc>,
    subject_kind: &str,
    subject_id: String,
) -> TimelineEntry {
    TimelineEntry {
        id: Uuid::new_v4(),
        session_id,
        stage,
        label,
        timestamp,
        offset_seconds: (timestamp - reference).num_milliseconds() as f64 / 1000.0,
        duration_ms: None,
        subject_kind: subject_kind.to_string(),
        subject_id,
        parallel_group: None,
        on_critical_path: false,
    }
}

fn assign_parallel_groups(entries: &mut [TimelineEntry]) {
    let mut i = 0;
    let mut group_counter = 0usize;
    while i < entries.len() {
        let mut j = i + 1;
        while j < entries.len()
            && entries[j].stage == entries[i].stage
            && (entries[j].timestamp - entries[i].timestamp).num_milliseconds() <= PARALLEL_WINDOW_MS
        {
            j += 1;
        }
        if j - i > 1 {
            let group = format!("group-{group_counter}");
            group_counter += 1;
            for entry in &mut entries[i..j] {
                entry.parallel_group = Some(group.clone());
            }
        }
        i = j;
    }
}
