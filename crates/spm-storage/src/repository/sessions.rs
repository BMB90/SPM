use rusqlite::{params, Connection, OptionalExtension};
use spm_core::{BootSession, Platform};
use uuid::Uuid;

use crate::error::{StorageError, StorageResult};
use crate::pagination::{Page, Pagination};

use super::util::{dt_from_sql, dt_to_sql, uuid_from_sql, uuid_to_sql};

pub fn insert(conn: &Connection, session: &BootSession) -> StorageResult<()> {
    conn.execute(
        "INSERT INTO boot_sessions
            (id, hostname, platform, os_version, boot_time, capture_started_at, capture_completed_at, spm_version, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            uuid_to_sql(&session.id),
            session.hostname,
            session.platform.to_string(),
            session.os_version,
            dt_to_sql(&session.boot_time),
            session.capture_started_at.to_rfc3339(),
            dt_to_sql(&session.capture_completed_at),
            session.spm_version,
            session.notes,
        ],
    )?;
    Ok(())
}

pub fn mark_completed(conn: &Connection, id: Uuid, completed_at: chrono::DateTime<chrono::Utc>) -> StorageResult<()> {
    conn.execute(
        "UPDATE boot_sessions SET capture_completed_at = ?1 WHERE id = ?2",
        params![completed_at.to_rfc3339(), uuid_to_sql(&id)],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: Uuid) -> StorageResult<BootSession> {
    conn.query_row(
        "SELECT id, hostname, platform, os_version, boot_time, capture_started_at, capture_completed_at, spm_version, notes
         FROM boot_sessions WHERE id = ?1",
        params![uuid_to_sql(&id)],
        row_to_session,
    )
    .optional()?
    .ok_or(StorageError::NotFound)
}

pub fn latest(conn: &Connection) -> StorageResult<Option<BootSession>> {
    conn.query_row(
        "SELECT id, hostname, platform, os_version, boot_time, capture_started_at, capture_completed_at, spm_version, notes
         FROM boot_sessions ORDER BY capture_started_at DESC LIMIT 1",
        [],
        row_to_session,
    )
    .optional()
    .map_err(Into::into)
}

pub fn list(conn: &Connection, pagination: Pagination) -> StorageResult<Page<BootSession>> {
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM boot_sessions", [], |r| r.get(0))?;
    let mut stmt = conn.prepare(
        "SELECT id, hostname, platform, os_version, boot_time, capture_started_at, capture_completed_at, spm_version, notes
         FROM boot_sessions ORDER BY capture_started_at DESC LIMIT ?1 OFFSET ?2",
    )?;
    let items = stmt
        .query_map(params![pagination.effective_limit(), pagination.effective_offset()], row_to_session)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Page { items, total, limit: pagination.limit, offset: pagination.offset })
}

pub fn delete(conn: &Connection, id: Uuid) -> StorageResult<()> {
    conn.execute("DELETE FROM boot_sessions WHERE id = ?1", params![uuid_to_sql(&id)])?;
    Ok(())
}

fn row_to_session(row: &rusqlite::Row) -> rusqlite::Result<BootSession> {
    let platform_str: String = row.get(2)?;
    let platform = match platform_str.as_str() {
        "windows" => Platform::Windows,
        "linux" => Platform::Linux,
        other => return Err(super::util::parse_err("platform", other)),
    };
    Ok(BootSession {
        id: uuid_from_sql(row.get(0)?)?,
        hostname: row.get(1)?,
        platform,
        os_version: row.get(3)?,
        boot_time: dt_from_sql(row.get(4)?)?,
        capture_started_at: dt_from_sql(row.get(5)?)?.unwrap_or_else(chrono::Utc::now),
        capture_completed_at: dt_from_sql(row.get(6)?)?,
        spm_version: row.get(7)?,
        notes: row.get(8)?,
    })
}
