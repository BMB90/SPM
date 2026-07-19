# Database Schema

SQLite, via `rusqlite` (bundled — no system SQLite dependency) + `r2d2`
connection pooling. Schema lives entirely in
`crates/spm-storage/src/migrations.rs` as versioned, append-only SQL
migrations (`schema_migrations` tracks which have run).

## Conventions

- Every entity table has `id TEXT PRIMARY KEY` (a UUID) and
  `session_id TEXT NOT NULL REFERENCES boot_sessions(id) ON DELETE CASCADE`
  — deleting a session deletes everything captured in it.
- Timestamps are stored as RFC3339 text (`TEXT`), never Julian/Unix
  numeric — human-readable in `sqlite3` CLI output, sorts correctly as
  text because RFC3339 is lexicographically ordered.
- Nested/variable-shape data (argument lists, environment maps, evidence
  arrays, attribute maps) is stored as a JSON `TEXT` column
  (`*_json` suffix) rather than a normalized child table — these fields
  are always read/written whole, never queried by sub-field, so
  normalizing them would add join complexity for no query benefit.
- Scalar fields that are filtered, sorted, or joined on (pid, name, path,
  state, timestamps) are real columns with indexes.

## Tables

| Table | Purpose | Key indexes |
|---|---|---|
| `boot_sessions` | One row per capture | — |
| `processes` | `ProcessInfo` | `(session_id)`, `(session_id, pid)`, `(sha256)`, `(executable_path)` |
| `services` | `ServiceInfo` | `(session_id)`, `(session_id, name)` |
| `drivers` | `DriverInfo` | `(session_id)` |
| `modules` | `ModuleInfo` (DLLs/shared libs per process) | `(session_id)`, `(session_id, parent_pid)` |
| `file_activity` | `FileActivity` | `(session_id)`, `(path)` |
| `network_activity` | `NetworkActivity` | `(session_id)`, `(remote_address, remote_port)` |
| `config_entries` | `ConfigEntry` (registry/startup-folder/scheduled-task evidence) | `(session_id)`, `(location)` |
| `timeline_entries` | `TimelineEntry` | `(session_id)`, `(session_id, offset_seconds)` |
| `graph_nodes` / `graph_edges` | `DependencyGraph` | `(session_id, id)` PK on nodes; `(session_id)` on edges |

## Enum encoding

Rust enums (e.g. `ProcessRole`, `SignatureStatus`, `ServiceState`,
`BootStage`) are stored as their `snake_case` string form in a `TEXT`
column — see the `*_to_str`/`*_from_str` pairs in each
`crates/spm-storage/src/repository/*.rs` file. This keeps raw `sqlite3`
queries against the database human-readable (`WHERE role = 'service'`)
without needing to know the enum's discriminant order.

## Migrating

Add a new `(version, sql)` tuple to the `MIGRATIONS` slice in
`migrations.rs` — **never edit an already-shipped entry**, since it may
already be applied on a user's database file. `apply_migrations` runs
each pending migration in its own transaction and records it in
`schema_migrations`.

## Swapping in PostgreSQL later

The entire schema/query surface is behind the `Storage` facade
(`crates/spm-storage/src/storage.rs`) and the `repository/*` modules. A
Postgres backend would add a parallel `db_postgres.rs` +
`repository_postgres/` implementing the same `Storage` public API (or
have `Storage` become a trait with two impls) — nothing outside
`spm-storage` should need to change, since callers only ever see
`Storage`'s methods and the `spm_core` domain types.
