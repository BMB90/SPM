use rusqlite::params;
use spm_core::{FileActivity, FileOperation};
use uuid::Uuid;

use crate::error::StorageResult;
use crate::pagination::{Page, Pagination};

use super::util::uuid_from_sql;
use super::util::uuid_to_sql;

const COLUMNS: &str = "id, session_id, operation, path, new_path, owner, pid, process_executable, timestamp";

pub fn insert(conn: &rusqlite::Connection, f: &FileActivity) -> StorageResult<()> {
    conn.execute(
        &format!("INSERT INTO file_activity ({COLUMNS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)"),
        params![
            uuid_to_sql(&f.id),
            uuid_to_sql(&f.session_id),
            op_to_str(&f.operation),
            f.path,
            f.new_path,
            f.owner,
            f.pid,
            f.process_executable,
            f.timestamp.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn insert_many(conn: &mut rusqlite::Connection, items: &[FileActivity]) -> StorageResult<()> {
    let tx = conn.transaction()?;
    for f in items {
        insert(&tx, f)?;
    }
    tx.commit()?;
    Ok(())
}

pub fn list(conn: &rusqlite::Connection, session_id: Uuid, pagination: Pagination) -> StorageResult<Page<FileActivity>> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM file_activity WHERE session_id = ?1",
        params![uuid_to_sql(&session_id)],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!("SELECT {COLUMNS} FROM file_activity WHERE session_id = ?1 ORDER BY timestamp LIMIT ?2 OFFSET ?3"))?;
    let items = stmt
        .query_map(
            params![uuid_to_sql(&session_id), pagination.effective_limit(), pagination.effective_offset()],
            row_to_file_activity,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Page { items, total, limit: pagination.limit, offset: pagination.offset })
}

fn row_to_file_activity(row: &rusqlite::Row) -> rusqlite::Result<FileActivity> {
    let op_str: String = row.get(2)?;
    let ts: String = row.get(8)?;
    Ok(FileActivity {
        id: uuid_from_sql(row.get(0)?)?,
        session_id: uuid_from_sql(row.get(1)?)?,
        operation: op_from_str(&op_str)?,
        path: row.get(3)?,
        new_path: row.get(4)?,
        owner: row.get(5)?,
        pid: row.get(6)?,
        process_executable: row.get(7)?,
        timestamp: chrono::DateTime::parse_from_rfc3339(&ts)
            .map(|d| d.with_timezone(&chrono::Utc))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(e)))?,
    })
}

fn op_to_str(o: &FileOperation) -> &'static str {
    match o {
        FileOperation::Read => "read",
        FileOperation::Write => "write",
        FileOperation::Create => "create",
        FileOperation::Delete => "delete",
        FileOperation::Rename => "rename",
        FileOperation::PermissionChange => "permission_change",
        FileOperation::OwnerChange => "owner_change",
    }
}
fn op_from_str(s: &str) -> rusqlite::Result<FileOperation> {
    Ok(match s {
        "read" => FileOperation::Read,
        "write" => FileOperation::Write,
        "create" => FileOperation::Create,
        "delete" => FileOperation::Delete,
        "rename" => FileOperation::Rename,
        "permission_change" => FileOperation::PermissionChange,
        "owner_change" => FileOperation::OwnerChange,
        other => return Err(super::util::parse_err("file_operation", other)),
    })
}
