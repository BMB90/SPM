use rusqlite::params;
use spm_core::{ConfigAccess, ConfigEntry, ConfigEntryKind};
use uuid::Uuid;

use crate::error::StorageResult;
use crate::pagination::{Page, Pagination};

use super::util::{json_from_sql, json_to_sql, uuid_from_sql, uuid_to_sql};

const COLUMNS: &str = "id, session_id, kind, location, name, value, access, pid, related_startup_items_json";

pub fn insert(conn: &rusqlite::Connection, c: &ConfigEntry) -> StorageResult<()> {
    conn.execute(
        &format!("INSERT INTO config_entries ({COLUMNS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)"),
        params![
            uuid_to_sql(&c.id),
            uuid_to_sql(&c.session_id),
            kind_to_str(&c.kind),
            c.location,
            c.name,
            c.value,
            access_to_str(&c.access),
            c.pid,
            json_to_sql(&c.related_startup_items)?,
        ],
    )?;
    Ok(())
}

pub fn insert_many(conn: &mut rusqlite::Connection, items: &[ConfigEntry]) -> StorageResult<()> {
    let tx = conn.transaction()?;
    for c in items {
        insert(&tx, c)?;
    }
    tx.commit()?;
    Ok(())
}

pub fn list(conn: &rusqlite::Connection, session_id: Uuid, pagination: Pagination) -> StorageResult<Page<ConfigEntry>> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM config_entries WHERE session_id = ?1",
        params![uuid_to_sql(&session_id)],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!("SELECT {COLUMNS} FROM config_entries WHERE session_id = ?1 ORDER BY location LIMIT ?2 OFFSET ?3"))?;
    let items = stmt
        .query_map(
            params![uuid_to_sql(&session_id), pagination.effective_limit(), pagination.effective_offset()],
            row_to_config_entry,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Page { items, total, limit: pagination.limit, offset: pagination.offset })
}

fn row_to_config_entry(row: &rusqlite::Row) -> rusqlite::Result<ConfigEntry> {
    let kind_str: String = row.get(2)?;
    let access_str: String = row.get(6)?;
    let related_json: String = row.get(8)?;
    Ok(ConfigEntry {
        id: uuid_from_sql(row.get(0)?)?,
        session_id: uuid_from_sql(row.get(1)?)?,
        kind: kind_from_str(&kind_str)?,
        location: row.get(3)?,
        name: row.get(4)?,
        value: row.get(5)?,
        access: access_from_str(&access_str)?,
        pid: row.get(7)?,
        related_startup_items: json_from_sql(&related_json)?,
    })
}

fn kind_to_str(k: &ConfigEntryKind) -> &'static str {
    match k {
        ConfigEntryKind::RegistryKey => "registry_key",
        ConfigEntryKind::RegistryValue => "registry_value",
        ConfigEntryKind::ComRegistration => "com_registration",
        ConfigEntryKind::WindowsPolicy => "windows_policy",
        ConfigEntryKind::SystemdUnitFile => "systemd_unit_file",
        ConfigEntryKind::EnvironmentFile => "environment_file",
        ConfigEntryKind::UdevRule => "udev_rule",
        ConfigEntryKind::ModprobeConfig => "modprobe_config",
        ConfigEntryKind::KernelParameter => "kernel_parameter",
        ConfigEntryKind::GenericConfigFile => "generic_config_file",
    }
}
fn kind_from_str(s: &str) -> rusqlite::Result<ConfigEntryKind> {
    Ok(match s {
        "registry_key" => ConfigEntryKind::RegistryKey,
        "registry_value" => ConfigEntryKind::RegistryValue,
        "com_registration" => ConfigEntryKind::ComRegistration,
        "windows_policy" => ConfigEntryKind::WindowsPolicy,
        "systemd_unit_file" => ConfigEntryKind::SystemdUnitFile,
        "environment_file" => ConfigEntryKind::EnvironmentFile,
        "udev_rule" => ConfigEntryKind::UdevRule,
        "modprobe_config" => ConfigEntryKind::ModprobeConfig,
        "kernel_parameter" => ConfigEntryKind::KernelParameter,
        _ => ConfigEntryKind::GenericConfigFile,
    })
}
fn access_to_str(a: &ConfigAccess) -> &'static str {
    match a {
        ConfigAccess::Read => "read",
        ConfigAccess::Write => "write",
        ConfigAccess::Created => "created",
        ConfigAccess::Deleted => "deleted",
    }
}
fn access_from_str(s: &str) -> rusqlite::Result<ConfigAccess> {
    Ok(match s {
        "read" => ConfigAccess::Read,
        "write" => ConfigAccess::Write,
        "created" => ConfigAccess::Created,
        "deleted" => ConfigAccess::Deleted,
        other => return Err(super::util::parse_err("config_access", other)),
    })
}
