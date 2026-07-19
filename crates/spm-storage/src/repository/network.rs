use rusqlite::params;
use spm_core::{NetworkActivity, NetworkProtocol};
use uuid::Uuid;

use crate::error::StorageResult;
use crate::pagination::{Page, Pagination};

use super::util::{dt_from_sql, dt_to_sql, uuid_from_sql, uuid_to_sql};

const COLUMNS: &str = "id, session_id, pid, process_executable, protocol, local_address, local_port,
    remote_address, remote_port, dns_query, bytes_sent, bytes_received, tls_version, tls_sni, started_at, ended_at";

pub fn insert(conn: &rusqlite::Connection, n: &NetworkActivity) -> StorageResult<()> {
    conn.execute(
        &format!("INSERT INTO network_activity ({COLUMNS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)"),
        params![
            uuid_to_sql(&n.id),
            uuid_to_sql(&n.session_id),
            n.pid,
            n.process_executable,
            protocol_to_str(&n.protocol),
            n.local_address,
            n.local_port,
            n.remote_address,
            n.remote_port,
            n.dns_query,
            n.bytes_sent,
            n.bytes_received,
            n.tls_version,
            n.tls_sni,
            n.started_at.to_rfc3339(),
            dt_to_sql(&n.ended_at),
        ],
    )?;
    Ok(())
}

pub fn insert_many(conn: &mut rusqlite::Connection, items: &[NetworkActivity]) -> StorageResult<()> {
    let tx = conn.transaction()?;
    for n in items {
        insert(&tx, n)?;
    }
    tx.commit()?;
    Ok(())
}

pub fn list(conn: &rusqlite::Connection, session_id: Uuid, pagination: Pagination) -> StorageResult<Page<NetworkActivity>> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM network_activity WHERE session_id = ?1",
        params![uuid_to_sql(&session_id)],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!("SELECT {COLUMNS} FROM network_activity WHERE session_id = ?1 ORDER BY started_at LIMIT ?2 OFFSET ?3"))?;
    let items = stmt
        .query_map(
            params![uuid_to_sql(&session_id), pagination.effective_limit(), pagination.effective_offset()],
            row_to_network,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Page { items, total, limit: pagination.limit, offset: pagination.offset })
}

fn row_to_network(row: &rusqlite::Row) -> rusqlite::Result<NetworkActivity> {
    let protocol_str: String = row.get(4)?;
    let started_at: String = row.get(14)?;
    Ok(NetworkActivity {
        id: uuid_from_sql(row.get(0)?)?,
        session_id: uuid_from_sql(row.get(1)?)?,
        pid: row.get(2)?,
        process_executable: row.get(3)?,
        protocol: protocol_from_str(&protocol_str)?,
        local_address: row.get(5)?,
        local_port: row.get(6)?,
        remote_address: row.get(7)?,
        remote_port: row.get(8)?,
        dns_query: row.get(9)?,
        bytes_sent: row.get(10)?,
        bytes_received: row.get(11)?,
        tls_version: row.get(12)?,
        tls_sni: row.get(13)?,
        started_at: chrono::DateTime::parse_from_rfc3339(&started_at)
            .map(|d| d.with_timezone(&chrono::Utc))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(14, rusqlite::types::Type::Text, Box::new(e)))?,
        ended_at: dt_from_sql(row.get(15)?)?,
    })
}

fn protocol_to_str(p: &NetworkProtocol) -> &'static str {
    match p {
        NetworkProtocol::Tcp => "TCP",
        NetworkProtocol::Udp => "UDP",
        NetworkProtocol::Unix => "UNIX",
        NetworkProtocol::Other => "OTHER",
    }
}
fn protocol_from_str(s: &str) -> rusqlite::Result<NetworkProtocol> {
    Ok(match s {
        "TCP" => NetworkProtocol::Tcp,
        "UDP" => NetworkProtocol::Udp,
        "UNIX" => NetworkProtocol::Unix,
        _ => NetworkProtocol::Other,
    })
}
