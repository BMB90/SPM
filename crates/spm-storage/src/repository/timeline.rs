use rusqlite::params;
use spm_core::{BootStage, TimelineEntry};
use uuid::Uuid;

use crate::error::StorageResult;

use super::util::uuid_from_sql;
use super::util::uuid_to_sql;

const COLUMNS: &str = "id, session_id, stage, label, timestamp, offset_seconds, duration_ms, subject_kind,
    subject_id, parallel_group, on_critical_path";

pub fn insert(conn: &rusqlite::Connection, e: &TimelineEntry) -> StorageResult<()> {
    conn.execute(
        &format!("INSERT INTO timeline_entries ({COLUMNS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)"),
        params![
            uuid_to_sql(&e.id),
            uuid_to_sql(&e.session_id),
            stage_to_str(&e.stage),
            e.label,
            e.timestamp.to_rfc3339(),
            e.offset_seconds,
            e.duration_ms,
            e.subject_kind,
            e.subject_id,
            e.parallel_group,
            e.on_critical_path as i64,
        ],
    )?;
    Ok(())
}

pub fn insert_many(conn: &mut rusqlite::Connection, items: &[TimelineEntry]) -> StorageResult<()> {
    let tx = conn.transaction()?;
    for e in items {
        insert(&tx, e)?;
    }
    tx.commit()?;
    Ok(())
}

/// The full timeline for a session, ordered by offset. Timelines are
/// rendered as one continuous view (not paginated) since the UI needs the
/// whole picture to zoom/filter client-side.
pub fn list_all(conn: &rusqlite::Connection, session_id: Uuid) -> StorageResult<Vec<TimelineEntry>> {
    let mut stmt = conn.prepare(&format!("SELECT {COLUMNS} FROM timeline_entries WHERE session_id = ?1 ORDER BY offset_seconds ASC"))?;
    let items = stmt
        .query_map(params![uuid_to_sql(&session_id)], row_to_entry)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<TimelineEntry> {
    let stage_str: String = row.get(2)?;
    let ts: String = row.get(4)?;
    let on_critical_path: i64 = row.get(10)?;
    Ok(TimelineEntry {
        id: uuid_from_sql(row.get(0)?)?,
        session_id: uuid_from_sql(row.get(1)?)?,
        stage: stage_from_str(&stage_str)?,
        label: row.get(3)?,
        timestamp: chrono::DateTime::parse_from_rfc3339(&ts)
            .map(|d| d.with_timezone(&chrono::Utc))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e)))?,
        offset_seconds: row.get(5)?,
        duration_ms: row.get(6)?,
        subject_kind: row.get(7)?,
        subject_id: row.get(8)?,
        parallel_group: row.get(9)?,
        on_critical_path: on_critical_path != 0,
    })
}

fn stage_to_str(s: &BootStage) -> String {
    s.to_string()
}
fn stage_from_str(s: &str) -> rusqlite::Result<BootStage> {
    Ok(match s {
        "firmware" => BootStage::Firmware,
        "bootloader" => BootStage::Bootloader,
        "kernel" => BootStage::Kernel,
        "driver_init" => BootStage::DriverInit,
        "filesystem_mount" => BootStage::FilesystemMount,
        "device_discovery" => BootStage::DeviceDiscovery,
        "service_startup" => BootStage::ServiceStartup,
        "network_init" => BootStage::NetworkInit,
        "login_manager" => BootStage::LoginManager,
        "user_login" => BootStage::UserLogin,
        "desktop_init" => BootStage::DesktopInit,
        "startup_applications" => BootStage::StartupApplications,
        "scheduled_tasks" => BootStage::ScheduledTasks,
        "background_daemons" => BootStage::BackgroundDaemons,
        "desktop_ready" => BootStage::DesktopReady,
        "idle" => BootStage::Idle,
        _ => BootStage::Unknown,
    })
}
