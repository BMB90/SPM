use rusqlite::{params, Connection, OptionalExtension};
use spm_core::{ExecutableMetadata, PerformanceMetrics, ProcessInfo, ProcessRole, SecurityInfo, SignatureStatus, StartupSource};
use uuid::Uuid;

use crate::error::{StorageError, StorageResult};
use crate::pagination::{Page, Pagination};

use super::util::{dt_from_sql, dt_to_sql, json_from_sql, json_to_sql, uuid_from_sql, uuid_to_sql};

const COLUMNS: &str = "id, session_id, pid, ppid, executable_name, executable_path, working_directory, command_line,
    arguments_json, environment_json, start_time, exit_time, exit_code, user_name, group_name, thread_count,
    handle_count, sha256, signature_status, signer, metadata_json, role, owning_service, startup_source_json,
    security_json, performance_json";

pub fn insert(conn: &Connection, p: &ProcessInfo) -> StorageResult<()> {
    conn.execute(
        &format!("INSERT INTO processes ({COLUMNS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26)"),
        params![
            uuid_to_sql(&p.id),
            uuid_to_sql(&p.session_id),
            p.pid,
            p.ppid,
            p.executable_name,
            p.executable_path,
            p.working_directory,
            p.command_line,
            json_to_sql(&p.arguments)?,
            json_to_sql(&p.environment)?,
            dt_to_sql(&p.start_time),
            dt_to_sql(&p.exit_time),
            p.exit_code,
            p.user,
            p.group,
            p.thread_count,
            p.handle_count,
            p.sha256,
            signature_status_to_str(&p.signature_status),
            p.signer,
            json_to_sql(&p.metadata)?,
            role_to_str(&p.role),
            p.owning_service,
            p.startup_source.as_ref().map(json_to_sql).transpose()?,
            json_to_sql(&p.security)?,
            json_to_sql(&p.performance)?,
        ],
    )?;
    Ok(())
}

pub fn insert_many(conn: &mut Connection, items: &[ProcessInfo]) -> StorageResult<()> {
    let tx = conn.transaction()?;
    for p in items {
        insert(&tx, p)?;
    }
    tx.commit()?;
    Ok(())
}

pub fn get(conn: &Connection, id: Uuid) -> StorageResult<ProcessInfo> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM processes WHERE id = ?1"),
        params![uuid_to_sql(&id)],
        row_to_process,
    )
    .optional()?
    .ok_or(StorageError::NotFound)
}

#[derive(Debug, Clone, Default)]
pub struct ProcessFilter {
    pub pid: Option<u32>,
    pub name_contains: Option<String>,
    pub user: Option<String>,
    pub role: Option<ProcessRole>,
    pub signed_only: Option<bool>,
}

pub fn list(
    conn: &Connection,
    session_id: Uuid,
    filter: &ProcessFilter,
    pagination: Pagination,
) -> StorageResult<Page<ProcessInfo>> {
    let mut where_clauses = vec!["session_id = ?1".to_string()];
    let mut idx = 2;
    let mut bind_strings: Vec<String> = Vec::new();

    if filter.pid.is_some() {
        where_clauses.push(format!("pid = ?{idx}"));
        idx += 1;
    }
    if let Some(name) = &filter.name_contains {
        where_clauses.push(format!("executable_name LIKE ?{idx}"));
        bind_strings.push(format!("%{name}%"));
        idx += 1;
    }
    if let Some(user) = &filter.user {
        where_clauses.push(format!("user_name = ?{idx}"));
        bind_strings.push(user.clone());
        idx += 1;
    }
    if let Some(role) = &filter.role {
        where_clauses.push(format!("role = ?{idx}"));
        bind_strings.push(role_to_str(role).to_string());
        idx += 1;
    }
    if let Some(signed_only) = filter.signed_only {
        if signed_only {
            where_clauses.push("signature_status = 'signed'".to_string());
        } else {
            where_clauses.push("signature_status != 'signed'".to_string());
        }
    }
    let _ = idx;

    let where_sql = where_clauses.join(" AND ");

    // Build params dynamically: session_id, then pid (if any), then the
    // pre-formatted string binds in the order they were pushed.
    let mut owned_params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(uuid_to_sql(&session_id))];
    if let Some(pid) = filter.pid {
        owned_params.push(Box::new(pid));
    }
    for s in &bind_strings {
        owned_params.push(Box::new(s.clone()));
    }
    let param_refs: Vec<&dyn rusqlite::ToSql> = owned_params.iter().map(|b| b.as_ref()).collect();

    let count_sql = format!("SELECT COUNT(*) FROM processes WHERE {where_sql}");
    let total: i64 = conn.query_row(&count_sql, param_refs.as_slice(), |r| r.get(0))?;

    let list_sql = format!(
        "SELECT {COLUMNS} FROM processes WHERE {where_sql} ORDER BY start_time ASC LIMIT ?{a} OFFSET ?{b}",
        a = param_refs.len() + 1,
        b = param_refs.len() + 2
    );
    let mut all_params = owned_params;
    all_params.push(Box::new(pagination.effective_limit()));
    all_params.push(Box::new(pagination.effective_offset()));
    let all_refs: Vec<&dyn rusqlite::ToSql> = all_params.iter().map(|b| b.as_ref()).collect();

    let mut stmt = conn.prepare(&list_sql)?;
    let items = stmt.query_map(all_refs.as_slice(), row_to_process)?.collect::<Result<Vec<_>, _>>()?;

    Ok(Page { items, total, limit: pagination.limit, offset: pagination.offset })
}

/// Full-text-ish search across name, path, command line, sha256, and pid
/// (as a string), used by the global search bar.
pub fn search(conn: &Connection, session_id: Uuid, query: &str, pagination: Pagination) -> StorageResult<Page<ProcessInfo>> {
    let like = format!("%{query}%");
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM processes WHERE session_id = ?1 AND
            (executable_name LIKE ?2 OR executable_path LIKE ?2 OR command_line LIKE ?2 OR sha256 LIKE ?2 OR CAST(pid AS TEXT) LIKE ?2)",
        params![uuid_to_sql(&session_id), like],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM processes WHERE session_id = ?1 AND
            (executable_name LIKE ?2 OR executable_path LIKE ?2 OR command_line LIKE ?2 OR sha256 LIKE ?2 OR CAST(pid AS TEXT) LIKE ?2)
         ORDER BY start_time ASC LIMIT ?3 OFFSET ?4"
    ))?;
    let items = stmt
        .query_map(
            params![uuid_to_sql(&session_id), like, pagination.effective_limit(), pagination.effective_offset()],
            row_to_process,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Page { items, total, limit: pagination.limit, offset: pagination.offset })
}

fn row_to_process(row: &rusqlite::Row) -> rusqlite::Result<ProcessInfo> {
    let arguments_json: String = row.get(8)?;
    let environment_json: String = row.get(9)?;
    let metadata_json: String = row.get(20)?;
    let startup_source_json: Option<String> = row.get(23)?;
    let security_json: String = row.get(24)?;
    let performance_json: String = row.get(25)?;
    let signature_status_str: String = row.get(18)?;
    let role_str: String = row.get(21)?;

    Ok(ProcessInfo {
        id: uuid_from_sql(row.get(0)?)?,
        session_id: uuid_from_sql(row.get(1)?)?,
        pid: row.get(2)?,
        ppid: row.get(3)?,
        executable_name: row.get(4)?,
        executable_path: row.get(5)?,
        working_directory: row.get(6)?,
        command_line: row.get(7)?,
        arguments: json_from_sql(&arguments_json)?,
        environment: json_from_sql(&environment_json)?,
        start_time: dt_from_sql(row.get(10)?)?,
        exit_time: dt_from_sql(row.get(11)?)?,
        exit_code: row.get(12)?,
        user: row.get(13)?,
        group: row.get(14)?,
        thread_count: row.get(15)?,
        handle_count: row.get(16)?,
        sha256: row.get(17)?,
        signature_status: signature_status_from_str(&signature_status_str)?,
        signer: row.get(19)?,
        metadata: json_from_sql::<ExecutableMetadata>(&metadata_json)?,
        role: role_from_str(&role_str)?,
        owning_service: row.get(22)?,
        startup_source: startup_source_json.map(|s| json_from_sql::<StartupSource>(&s)).transpose()?,
        security: json_from_sql::<SecurityInfo>(&security_json)?,
        performance: json_from_sql::<PerformanceMetrics>(&performance_json)?,
    })
}

fn signature_status_to_str(s: &SignatureStatus) -> &'static str {
    match s {
        SignatureStatus::Signed => "signed",
        SignatureStatus::SignedUntrusted => "signed_untrusted",
        SignatureStatus::Unsigned => "unsigned",
        SignatureStatus::Unknown => "unknown",
    }
}

fn signature_status_from_str(s: &str) -> rusqlite::Result<SignatureStatus> {
    Ok(match s {
        "signed" => SignatureStatus::Signed,
        "signed_untrusted" => SignatureStatus::SignedUntrusted,
        "unsigned" => SignatureStatus::Unsigned,
        _ => SignatureStatus::Unknown,
    })
}

fn role_to_str(r: &ProcessRole) -> &'static str {
    match r {
        ProcessRole::KernelProcess => "kernel_process",
        ProcessRole::System => "system",
        ProcessRole::Service => "service",
        ProcessRole::Daemon => "daemon",
        ProcessRole::ScheduledTask => "scheduled_task",
        ProcessRole::LoginItem => "login_item",
        ProcessRole::UserApplication => "user_application",
        ProcessRole::Unknown => "unknown",
    }
}

fn role_from_str(s: &str) -> rusqlite::Result<ProcessRole> {
    Ok(match s {
        "kernel_process" => ProcessRole::KernelProcess,
        "system" => ProcessRole::System,
        "service" => ProcessRole::Service,
        "daemon" => ProcessRole::Daemon,
        "scheduled_task" => ProcessRole::ScheduledTask,
        "login_item" => ProcessRole::LoginItem,
        "user_application" => ProcessRole::UserApplication,
        _ => ProcessRole::Unknown,
    })
}
