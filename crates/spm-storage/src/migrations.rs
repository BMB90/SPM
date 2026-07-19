use rusqlite::Connection;

use crate::error::StorageResult;

/// Schema history. Each entry is applied once, in order, inside a
/// transaction, and recorded in `schema_migrations`. Append new
/// migrations to the end of this slice — never edit an already-shipped
/// entry, since it may already be applied on a user's database.
const MIGRATIONS: &[(i64, &str)] = &[(1, MIGRATION_0001)];

const MIGRATION_0001: &str = r#"
CREATE TABLE boot_sessions (
    id                    TEXT PRIMARY KEY,
    hostname              TEXT NOT NULL,
    platform              TEXT NOT NULL,
    os_version            TEXT NOT NULL,
    boot_time             TEXT,
    capture_started_at    TEXT NOT NULL,
    capture_completed_at  TEXT,
    spm_version           TEXT NOT NULL,
    notes                 TEXT
);

CREATE TABLE processes (
    id                  TEXT PRIMARY KEY,
    session_id          TEXT NOT NULL REFERENCES boot_sessions(id) ON DELETE CASCADE,
    pid                 INTEGER NOT NULL,
    ppid                INTEGER,
    executable_name     TEXT NOT NULL,
    executable_path     TEXT,
    working_directory   TEXT,
    command_line        TEXT,
    arguments_json      TEXT NOT NULL DEFAULT '[]',
    environment_json    TEXT NOT NULL DEFAULT '{}',
    start_time          TEXT,
    exit_time           TEXT,
    exit_code           INTEGER,
    user_name           TEXT,
    group_name          TEXT,
    thread_count        INTEGER,
    handle_count        INTEGER,
    sha256              TEXT,
    signature_status    TEXT NOT NULL DEFAULT 'unknown',
    signer              TEXT,
    metadata_json       TEXT NOT NULL DEFAULT '{}',
    role                TEXT NOT NULL DEFAULT 'unknown',
    owning_service      TEXT,
    startup_source_json TEXT,
    security_json       TEXT NOT NULL DEFAULT '{}',
    performance_json    TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX idx_processes_session ON processes(session_id);
CREATE INDEX idx_processes_pid ON processes(session_id, pid);
CREATE INDEX idx_processes_sha256 ON processes(sha256);
CREATE INDEX idx_processes_path ON processes(executable_path);

CREATE TABLE services (
    id                TEXT PRIMARY KEY,
    session_id        TEXT NOT NULL REFERENCES boot_sessions(id) ON DELETE CASCADE,
    name              TEXT NOT NULL,
    display_name      TEXT,
    description       TEXT,
    binary_path       TEXT,
    config_path       TEXT,
    state             TEXT NOT NULL DEFAULT 'unknown',
    start_type        TEXT NOT NULL DEFAULT 'unknown',
    owner             TEXT,
    pid               INTEGER,
    depends_on_json   TEXT NOT NULL DEFAULT '[]',
    required_by_json  TEXT NOT NULL DEFAULT '[]',
    start_time        TEXT,
    end_time          TEXT,
    restart_count     INTEGER NOT NULL DEFAULT 0,
    last_failure      TEXT,
    performance_json  TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX idx_services_session ON services(session_id);
CREATE INDEX idx_services_name ON services(session_id, name);

CREATE TABLE drivers (
    id               TEXT PRIMARY KEY,
    session_id       TEXT NOT NULL REFERENCES boot_sessions(id) ON DELETE CASCADE,
    name             TEXT NOT NULL,
    path             TEXT,
    load_order       INTEGER,
    load_time        TEXT,
    unload_time      TEXT,
    version          TEXT,
    vendor           TEXT,
    signature_status TEXT NOT NULL DEFAULT 'unknown',
    depends_on_json  TEXT NOT NULL DEFAULT '[]',
    status           TEXT NOT NULL DEFAULT 'unknown',
    failure_reason   TEXT
);
CREATE INDEX idx_drivers_session ON drivers(session_id);

CREATE TABLE modules (
    id               TEXT PRIMARY KEY,
    session_id       TEXT NOT NULL REFERENCES boot_sessions(id) ON DELETE CASCADE,
    kind             TEXT NOT NULL,
    name             TEXT NOT NULL,
    path             TEXT,
    version          TEXT,
    signature_status TEXT NOT NULL DEFAULT 'unknown',
    load_time        TEXT,
    parent_pid       INTEGER NOT NULL
);
CREATE INDEX idx_modules_session ON modules(session_id);
CREATE INDEX idx_modules_parent_pid ON modules(session_id, parent_pid);

CREATE TABLE file_activity (
    id                 TEXT PRIMARY KEY,
    session_id         TEXT NOT NULL REFERENCES boot_sessions(id) ON DELETE CASCADE,
    operation          TEXT NOT NULL,
    path               TEXT NOT NULL,
    new_path           TEXT,
    owner              TEXT,
    pid                INTEGER NOT NULL,
    process_executable TEXT,
    timestamp          TEXT NOT NULL
);
CREATE INDEX idx_file_activity_session ON file_activity(session_id);
CREATE INDEX idx_file_activity_path ON file_activity(path);

CREATE TABLE network_activity (
    id                 TEXT PRIMARY KEY,
    session_id         TEXT NOT NULL REFERENCES boot_sessions(id) ON DELETE CASCADE,
    pid                INTEGER NOT NULL,
    process_executable TEXT,
    protocol           TEXT NOT NULL,
    local_address      TEXT,
    local_port         INTEGER,
    remote_address     TEXT,
    remote_port        INTEGER,
    dns_query          TEXT,
    bytes_sent         INTEGER,
    bytes_received     INTEGER,
    tls_version        TEXT,
    tls_sni            TEXT,
    started_at         TEXT NOT NULL,
    ended_at           TEXT
);
CREATE INDEX idx_network_activity_session ON network_activity(session_id);
CREATE INDEX idx_network_activity_remote ON network_activity(remote_address, remote_port);

CREATE TABLE config_entries (
    id                        TEXT PRIMARY KEY,
    session_id                TEXT NOT NULL REFERENCES boot_sessions(id) ON DELETE CASCADE,
    kind                      TEXT NOT NULL,
    location                  TEXT NOT NULL,
    name                      TEXT,
    value                     TEXT,
    access                    TEXT NOT NULL,
    pid                       INTEGER,
    related_startup_items_json TEXT NOT NULL DEFAULT '[]'
);
CREATE INDEX idx_config_entries_session ON config_entries(session_id);
CREATE INDEX idx_config_entries_location ON config_entries(location);

CREATE TABLE timeline_entries (
    id               TEXT PRIMARY KEY,
    session_id       TEXT NOT NULL REFERENCES boot_sessions(id) ON DELETE CASCADE,
    stage            TEXT NOT NULL,
    label            TEXT NOT NULL,
    timestamp        TEXT NOT NULL,
    offset_seconds   REAL NOT NULL,
    duration_ms      INTEGER,
    subject_kind     TEXT NOT NULL,
    subject_id       TEXT NOT NULL,
    parallel_group   TEXT,
    on_critical_path INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_timeline_session ON timeline_entries(session_id);
CREATE INDEX idx_timeline_offset ON timeline_entries(session_id, offset_seconds);

CREATE TABLE graph_nodes (
    id             TEXT NOT NULL,
    session_id     TEXT NOT NULL REFERENCES boot_sessions(id) ON DELETE CASCADE,
    kind           TEXT NOT NULL,
    label          TEXT NOT NULL,
    attributes_json TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY (session_id, id)
);

CREATE TABLE graph_edges (
    id         TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES boot_sessions(id) ON DELETE CASCADE,
    from_node  TEXT NOT NULL,
    to_node    TEXT NOT NULL,
    kind       TEXT NOT NULL,
    evidence   TEXT
);
CREATE INDEX idx_graph_edges_session ON graph_edges(session_id);
"#;

pub fn apply_migrations(conn: &mut Connection) -> StorageResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
         PRAGMA foreign_keys = ON;",
    )?;

    let applied: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;

    for (version, sql) in MIGRATIONS {
        if *version <= applied {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, datetime('now'))",
            [version],
        )?;
        tx.commit()?;
        tracing::info!(version, "applied storage migration");
    }

    Ok(())
}
