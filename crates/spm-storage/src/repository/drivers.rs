use rusqlite::{params, Connection};
use spm_core::{DriverInfo, DriverStatus, SignatureStatus};
use uuid::Uuid;

use crate::error::StorageResult;
use crate::pagination::{Page, Pagination};

use super::util::{dt_from_sql, dt_to_sql, json_from_sql, json_to_sql, uuid_from_sql, uuid_to_sql};

const COLUMNS: &str = "id, session_id, name, path, load_order, load_time, unload_time, version, vendor,
    signature_status, depends_on_json, status, failure_reason";

pub fn insert(conn: &Connection, d: &DriverInfo) -> StorageResult<()> {
    conn.execute(
        &format!("INSERT INTO drivers ({COLUMNS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)"),
        params![
            uuid_to_sql(&d.id),
            uuid_to_sql(&d.session_id),
            d.name,
            d.path,
            d.load_order,
            dt_to_sql(&d.load_time),
            dt_to_sql(&d.unload_time),
            d.version,
            d.vendor,
            sig_to_str(&d.signature_status),
            json_to_sql(&d.depends_on)?,
            status_to_str(&d.status),
            d.failure_reason,
        ],
    )?;
    Ok(())
}

pub fn insert_many(conn: &mut Connection, items: &[DriverInfo]) -> StorageResult<()> {
    let tx = conn.transaction()?;
    for d in items {
        insert(&tx, d)?;
    }
    tx.commit()?;
    Ok(())
}

pub fn list(conn: &Connection, session_id: Uuid, pagination: Pagination) -> StorageResult<Page<DriverInfo>> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM drivers WHERE session_id = ?1",
        params![uuid_to_sql(&session_id)],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!("SELECT {COLUMNS} FROM drivers WHERE session_id = ?1 ORDER BY load_order LIMIT ?2 OFFSET ?3"))?;
    let items = stmt
        .query_map(
            params![uuid_to_sql(&session_id), pagination.effective_limit(), pagination.effective_offset()],
            row_to_driver,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Page { items, total, limit: pagination.limit, offset: pagination.offset })
}

fn row_to_driver(row: &rusqlite::Row) -> rusqlite::Result<DriverInfo> {
    let depends_on_json: String = row.get(10)?;
    let sig_str: String = row.get(9)?;
    let status_str: String = row.get(11)?;
    Ok(DriverInfo {
        id: uuid_from_sql(row.get(0)?)?,
        session_id: uuid_from_sql(row.get(1)?)?,
        name: row.get(2)?,
        path: row.get(3)?,
        load_order: row.get(4)?,
        load_time: dt_from_sql(row.get(5)?)?,
        unload_time: dt_from_sql(row.get(6)?)?,
        version: row.get(7)?,
        vendor: row.get(8)?,
        signature_status: sig_from_str(&sig_str)?,
        depends_on: json_from_sql(&depends_on_json)?,
        status: status_from_str(&status_str)?,
        failure_reason: row.get(12)?,
    })
}

fn sig_to_str(s: &SignatureStatus) -> &'static str {
    match s {
        SignatureStatus::Signed => "signed",
        SignatureStatus::SignedUntrusted => "signed_untrusted",
        SignatureStatus::Unsigned => "unsigned",
        SignatureStatus::Unknown => "unknown",
    }
}
fn sig_from_str(s: &str) -> rusqlite::Result<SignatureStatus> {
    Ok(match s {
        "signed" => SignatureStatus::Signed,
        "signed_untrusted" => SignatureStatus::SignedUntrusted,
        "unsigned" => SignatureStatus::Unsigned,
        _ => SignatureStatus::Unknown,
    })
}
fn status_to_str(s: &DriverStatus) -> &'static str {
    match s {
        DriverStatus::Running => "running",
        DriverStatus::Stopped => "stopped",
        DriverStatus::Failed => "failed",
        DriverStatus::Unknown => "unknown",
    }
}
fn status_from_str(s: &str) -> rusqlite::Result<DriverStatus> {
    Ok(match s {
        "running" => DriverStatus::Running,
        "stopped" => DriverStatus::Stopped,
        "failed" => DriverStatus::Failed,
        _ => DriverStatus::Unknown,
    })
}
