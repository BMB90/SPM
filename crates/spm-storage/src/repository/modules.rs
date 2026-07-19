use rusqlite::params;
use spm_core::{ModuleInfo, ModuleKind, SignatureStatus};
use uuid::Uuid;

use crate::error::StorageResult;
use crate::pagination::{Page, Pagination};

use super::util::{dt_from_sql, dt_to_sql, uuid_from_sql, uuid_to_sql};

const COLUMNS: &str = "id, session_id, kind, name, path, version, signature_status, load_time, parent_pid";

pub fn insert(conn: &rusqlite::Connection, m: &ModuleInfo) -> StorageResult<()> {
    conn.execute(
        &format!("INSERT INTO modules ({COLUMNS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)"),
        params![
            uuid_to_sql(&m.id),
            uuid_to_sql(&m.session_id),
            kind_to_str(&m.kind),
            m.name,
            m.path,
            m.version,
            sig_to_str(&m.signature_status),
            dt_to_sql(&m.load_time),
            m.parent_pid,
        ],
    )?;
    Ok(())
}

pub fn insert_many(conn: &mut rusqlite::Connection, items: &[ModuleInfo]) -> StorageResult<()> {
    let tx = conn.transaction()?;
    for m in items {
        insert(&tx, m)?;
    }
    tx.commit()?;
    Ok(())
}

pub fn list_for_process(conn: &rusqlite::Connection, session_id: Uuid, pid: u32, pagination: Pagination) -> StorageResult<Page<ModuleInfo>> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM modules WHERE session_id = ?1 AND parent_pid = ?2",
        params![uuid_to_sql(&session_id), pid],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM modules WHERE session_id = ?1 AND parent_pid = ?2 ORDER BY name LIMIT ?3 OFFSET ?4"
    ))?;
    let items = stmt
        .query_map(
            params![uuid_to_sql(&session_id), pid, pagination.effective_limit(), pagination.effective_offset()],
            row_to_module,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Page { items, total, limit: pagination.limit, offset: pagination.offset })
}

fn row_to_module(row: &rusqlite::Row) -> rusqlite::Result<ModuleInfo> {
    let kind_str: String = row.get(2)?;
    let sig_str: String = row.get(6)?;
    Ok(ModuleInfo {
        id: uuid_from_sql(row.get(0)?)?,
        session_id: uuid_from_sql(row.get(1)?)?,
        kind: kind_from_str(&kind_str)?,
        name: row.get(3)?,
        path: row.get(4)?,
        version: row.get(5)?,
        signature_status: sig_from_str(&sig_str)?,
        load_time: dt_from_sql(row.get(7)?)?,
        parent_pid: row.get(8)?,
    })
}

fn kind_to_str(k: &ModuleKind) -> &'static str {
    match k {
        ModuleKind::Dll => "dll",
        ModuleKind::SharedLibrary => "shared_library",
        ModuleKind::KernelModule => "kernel_module",
        ModuleKind::Plugin => "plugin",
        ModuleKind::DynamicModule => "dynamic_module",
    }
}
fn kind_from_str(s: &str) -> rusqlite::Result<ModuleKind> {
    Ok(match s {
        "dll" => ModuleKind::Dll,
        "shared_library" => ModuleKind::SharedLibrary,
        "kernel_module" => ModuleKind::KernelModule,
        "plugin" => ModuleKind::Plugin,
        _ => ModuleKind::DynamicModule,
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
