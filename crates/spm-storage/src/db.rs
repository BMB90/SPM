use std::path::Path;

use r2d2_sqlite::SqliteConnectionManager;

use crate::error::StorageResult;
use crate::migrations::apply_migrations;

pub type Pool = r2d2::Pool<SqliteConnectionManager>;
pub type PooledConnection = r2d2::PooledConnection<SqliteConnectionManager>;

/// Opens (creating if absent) the SQLite database at `path`, applies any
/// pending migrations, and returns a connection pool. Pass `:memory:` for
/// an ephemeral in-process database (used by tests and mock captures).
pub fn open(path: impl AsRef<Path>) -> StorageResult<Pool> {
    let manager = SqliteConnectionManager::file(path.as_ref()).with_init(|conn| {
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        Ok(())
    });
    let pool = r2d2::Pool::builder().max_size(8).build(manager)?;
    let mut conn = pool.get()?;
    apply_migrations(&mut conn)?;
    Ok(pool)
}

pub fn open_in_memory() -> StorageResult<Pool> {
    let manager = SqliteConnectionManager::memory();
    // A single-connection pool: SQLite `:memory:` databases are private to
    // the connection that created them, so pooling >1 connection would
    // give each caller an empty, unrelated database.
    let pool = r2d2::Pool::builder().max_size(1).build(manager)?;
    let mut conn = pool.get()?;
    apply_migrations(&mut conn)?;
    Ok(pool)
}
