use std::collections::{HashMap, HashSet};

use serde::Serialize;
use spm_storage::{Pagination, ProcessFilter, Storage, StorageResult};
use uuid::Uuid;

/// Historical database support: diffs two persisted sessions to answer
/// "what changed since last boot" — added/removed processes, added/removed
/// startup items, executable path drift (a classic persistence-swap
/// indicator), and boot-duration regression.
pub struct HistoricalComparator<'a> {
    storage: &'a Storage,
}

#[derive(Debug, Clone, Serialize)]
pub struct PathChange {
    pub executable_name: String,
    pub old_path: Option<String>,
    pub new_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SetDelta {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionComparison {
    pub baseline_session_id: Uuid,
    pub target_session_id: Uuid,
    pub processes: SetDelta,
    pub startup_items: SetDelta,
    pub executable_path_changes: Vec<PathChange>,
    pub boot_duration_seconds_baseline: Option<f64>,
    pub boot_duration_seconds_target: Option<f64>,
    pub boot_duration_seconds_delta: Option<f64>,
}

const COMPARISON_PAGE_SIZE: u32 = 5000;

impl<'a> HistoricalComparator<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    pub fn compare(&self, baseline: Uuid, target: Uuid) -> StorageResult<SessionComparison> {
        let page = Pagination::new(COMPARISON_PAGE_SIZE, 0);

        let baseline_procs = self.storage.list_processes(baseline, &ProcessFilter::default(), page)?.items;
        let target_procs = self.storage.list_processes(target, &ProcessFilter::default(), page)?.items;

        let baseline_names: HashSet<String> = baseline_procs.iter().map(|p| p.executable_name.clone()).collect();
        let target_names: HashSet<String> = target_procs.iter().map(|p| p.executable_name.clone()).collect();

        let mut baseline_paths: HashMap<String, String> = HashMap::new();
        for p in &baseline_procs {
            if let Some(path) = &p.executable_path {
                baseline_paths.insert(p.executable_name.clone(), path.clone());
            }
        }
        let mut path_changes = Vec::new();
        for p in &target_procs {
            if let (Some(new_path), Some(old_path)) = (&p.executable_path, baseline_paths.get(&p.executable_name)) {
                if new_path != old_path {
                    path_changes.push(PathChange {
                        executable_name: p.executable_name.clone(),
                        old_path: Some(old_path.clone()),
                        new_path: Some(new_path.clone()),
                    });
                }
            }
        }

        let baseline_config = self.storage.list_config_entries(baseline, page)?.items;
        let target_config = self.storage.list_config_entries(target, page)?.items;
        let key = |c: &spm_core::ConfigEntry| format!("{}::{}", c.location, c.name.clone().unwrap_or_default());
        let baseline_locations: HashSet<String> = baseline_config.iter().map(key).collect();
        let target_locations: HashSet<String> = target_config.iter().map(key).collect();

        let baseline_session = self.storage.get_session(baseline)?;
        let target_session = self.storage.get_session(target)?;
        let baseline_duration = session_duration_secs(&baseline_session);
        let target_duration = session_duration_secs(&target_session);

        Ok(SessionComparison {
            baseline_session_id: baseline,
            target_session_id: target,
            processes: SetDelta {
                added: target_names.difference(&baseline_names).cloned().collect(),
                removed: baseline_names.difference(&target_names).cloned().collect(),
            },
            startup_items: SetDelta {
                added: target_locations.difference(&baseline_locations).cloned().collect(),
                removed: baseline_locations.difference(&target_locations).cloned().collect(),
            },
            executable_path_changes: path_changes,
            boot_duration_seconds_baseline: baseline_duration,
            boot_duration_seconds_target: target_duration,
            boot_duration_seconds_delta: match (baseline_duration, target_duration) {
                (Some(a), Some(b)) => Some(b - a),
                _ => None,
            },
        })
    }
}

fn session_duration_secs(session: &spm_core::BootSession) -> Option<f64> {
    let end = session.capture_completed_at?;
    let start = session.boot_time.unwrap_or(session.capture_started_at);
    Some((end - start).num_milliseconds() as f64 / 1000.0)
}
