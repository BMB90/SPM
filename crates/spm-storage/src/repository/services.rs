use rusqlite::{params, Connection, OptionalExtension};
use spm_core::{PerformanceMetrics, ServiceInfo, ServiceStartType, ServiceState};
use uuid::Uuid;

use crate::error::{StorageError, StorageResult};
use crate::pagination::{Page, Pagination};

use super::util::{dt_from_sql, dt_to_sql, json_from_sql, json_to_sql, uuid_from_sql, uuid_to_sql};

const COLUMNS: &str = "id, session_id, name, display_name, description, binary_path, config_path, state,
    start_type, owner, pid, depends_on_json, required_by_json, start_time, end_time, restart_count,
    last_failure, performance_json";

pub fn insert(conn: &Connection, s: &ServiceInfo) -> StorageResult<()> {
    conn.execute(
        &format!("INSERT INTO services ({COLUMNS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)"),
        params![
            uuid_to_sql(&s.id),
            uuid_to_sql(&s.session_id),
            s.name,
            s.display_name,
            s.description,
            s.binary_path,
            s.config_path,
            state_to_str(&s.state),
            start_type_to_str(&s.start_type),
            s.owner,
            s.pid,
            json_to_sql(&s.depends_on)?,
            json_to_sql(&s.required_by)?,
            dt_to_sql(&s.start_time),
            dt_to_sql(&s.end_time),
            s.restart_count,
            s.last_failure,
            json_to_sql(&s.performance)?,
        ],
    )?;
    Ok(())
}

pub fn insert_many(conn: &mut Connection, items: &[ServiceInfo]) -> StorageResult<()> {
    let tx = conn.transaction()?;
    for s in items {
        insert(&tx, s)?;
    }
    tx.commit()?;
    Ok(())
}

pub fn get_by_name(conn: &Connection, session_id: Uuid, name: &str) -> StorageResult<ServiceInfo> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM services WHERE session_id = ?1 AND name = ?2"),
        params![uuid_to_sql(&session_id), name],
        row_to_service,
    )
    .optional()?
    .ok_or(StorageError::NotFound)
}

pub fn list(conn: &Connection, session_id: Uuid, pagination: Pagination) -> StorageResult<Page<ServiceInfo>> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM services WHERE session_id = ?1",
        params![uuid_to_sql(&session_id)],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!("SELECT {COLUMNS} FROM services WHERE session_id = ?1 ORDER BY name LIMIT ?2 OFFSET ?3"))?;
    let items = stmt
        .query_map(
            params![uuid_to_sql(&session_id), pagination.effective_limit(), pagination.effective_offset()],
            row_to_service,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Page { items, total, limit: pagination.limit, offset: pagination.offset })
}

fn row_to_service(row: &rusqlite::Row) -> rusqlite::Result<ServiceInfo> {
    let depends_on_json: String = row.get(11)?;
    let required_by_json: String = row.get(12)?;
    let performance_json: String = row.get(17)?;
    let state_str: String = row.get(7)?;
    let start_type_str: String = row.get(8)?;
    Ok(ServiceInfo {
        id: uuid_from_sql(row.get(0)?)?,
        session_id: uuid_from_sql(row.get(1)?)?,
        name: row.get(2)?,
        display_name: row.get(3)?,
        description: row.get(4)?,
        binary_path: row.get(5)?,
        config_path: row.get(6)?,
        state: state_from_str(&state_str)?,
        start_type: start_type_from_str(&start_type_str)?,
        owner: row.get(9)?,
        pid: row.get(10)?,
        depends_on: json_from_sql(&depends_on_json)?,
        required_by: json_from_sql(&required_by_json)?,
        start_time: dt_from_sql(row.get(13)?)?,
        end_time: dt_from_sql(row.get(14)?)?,
        restart_count: row.get(15)?,
        last_failure: row.get(16)?,
        performance: json_from_sql::<PerformanceMetrics>(&performance_json)?,
    })
}

fn state_to_str(s: &ServiceState) -> &'static str {
    match s {
        ServiceState::Running => "running",
        ServiceState::Stopped => "stopped",
        ServiceState::StartPending => "start_pending",
        ServiceState::StopPending => "stop_pending",
        ServiceState::Paused => "paused",
        ServiceState::Failed => "failed",
        ServiceState::Unknown => "unknown",
    }
}

fn state_from_str(s: &str) -> rusqlite::Result<ServiceState> {
    Ok(match s {
        "running" => ServiceState::Running,
        "stopped" => ServiceState::Stopped,
        "start_pending" => ServiceState::StartPending,
        "stop_pending" => ServiceState::StopPending,
        "paused" => ServiceState::Paused,
        "failed" => ServiceState::Failed,
        _ => ServiceState::Unknown,
    })
}

fn start_type_to_str(s: &ServiceStartType) -> &'static str {
    match s {
        ServiceStartType::Boot => "boot",
        ServiceStartType::System => "system",
        ServiceStartType::Automatic => "automatic",
        ServiceStartType::AutomaticDelayedStart => "automatic_delayed_start",
        ServiceStartType::Manual => "manual",
        ServiceStartType::Disabled => "disabled",
        ServiceStartType::Unknown => "unknown",
    }
}

fn start_type_from_str(s: &str) -> rusqlite::Result<ServiceStartType> {
    Ok(match s {
        "boot" => ServiceStartType::Boot,
        "system" => ServiceStartType::System,
        "automatic" => ServiceStartType::Automatic,
        "automatic_delayed_start" => ServiceStartType::AutomaticDelayedStart,
        "manual" => ServiceStartType::Manual,
        "disabled" => ServiceStartType::Disabled,
        _ => ServiceStartType::Unknown,
    })
}
