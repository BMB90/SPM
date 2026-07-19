use std::path::Path;

use spm_core::{
    BootSession, ConfigEntry, DependencyGraph, DriverInfo, FileActivity, ModuleInfo, NetworkActivity, ProcessInfo,
    ServiceInfo, TimelineEntry,
};
use uuid::Uuid;

use crate::db::{self, Pool};
use crate::error::StorageResult;
use crate::pagination::{Page, Pagination};
use crate::repository;
pub use crate::repository::processes::ProcessFilter;

/// Thread-safe facade over the SQLite-backed repositories. Cheap to clone
/// (wraps an `r2d2::Pool`); share one `Storage` across the API server, CLI,
/// and analysis engine.
#[derive(Clone)]
pub struct Storage {
    pool: Pool,
}

impl Storage {
    pub fn open(path: impl AsRef<Path>) -> StorageResult<Self> {
        Ok(Self { pool: db::open(path)? })
    }

    pub fn open_in_memory() -> StorageResult<Self> {
        Ok(Self { pool: db::open_in_memory()? })
    }

    // ---- Sessions ----------------------------------------------------

    pub fn create_session(&self, session: &BootSession) -> StorageResult<()> {
        let conn = self.pool.get()?;
        repository::sessions::insert(&conn, session)
    }

    pub fn complete_session(&self, id: Uuid, at: chrono::DateTime<chrono::Utc>) -> StorageResult<()> {
        let conn = self.pool.get()?;
        repository::sessions::mark_completed(&conn, id, at)
    }

    pub fn get_session(&self, id: Uuid) -> StorageResult<BootSession> {
        let conn = self.pool.get()?;
        repository::sessions::get(&conn, id)
    }

    pub fn latest_session(&self) -> StorageResult<Option<BootSession>> {
        let conn = self.pool.get()?;
        repository::sessions::latest(&conn)
    }

    pub fn list_sessions(&self, pagination: Pagination) -> StorageResult<Page<BootSession>> {
        let conn = self.pool.get()?;
        repository::sessions::list(&conn, pagination)
    }

    pub fn delete_session(&self, id: Uuid) -> StorageResult<()> {
        let conn = self.pool.get()?;
        repository::sessions::delete(&conn, id)
    }

    // ---- Processes -----------------------------------------------------

    pub fn save_processes(&self, items: &[ProcessInfo]) -> StorageResult<()> {
        let mut conn = self.pool.get()?;
        repository::processes::insert_many(&mut conn, items)
    }

    pub fn get_process(&self, id: Uuid) -> StorageResult<ProcessInfo> {
        let conn = self.pool.get()?;
        repository::processes::get(&conn, id)
    }

    pub fn list_processes(&self, session_id: Uuid, filter: &ProcessFilter, pagination: Pagination) -> StorageResult<Page<ProcessInfo>> {
        let conn = self.pool.get()?;
        repository::processes::list(&conn, session_id, filter, pagination)
    }

    pub fn search_processes(&self, session_id: Uuid, query: &str, pagination: Pagination) -> StorageResult<Page<ProcessInfo>> {
        let conn = self.pool.get()?;
        repository::processes::search(&conn, session_id, query, pagination)
    }

    // ---- Services -------------------------------------------------------

    pub fn save_services(&self, items: &[ServiceInfo]) -> StorageResult<()> {
        let mut conn = self.pool.get()?;
        repository::services::insert_many(&mut conn, items)
    }

    pub fn get_service(&self, session_id: Uuid, name: &str) -> StorageResult<ServiceInfo> {
        let conn = self.pool.get()?;
        repository::services::get_by_name(&conn, session_id, name)
    }

    pub fn list_services(&self, session_id: Uuid, pagination: Pagination) -> StorageResult<Page<ServiceInfo>> {
        let conn = self.pool.get()?;
        repository::services::list(&conn, session_id, pagination)
    }

    // ---- Drivers --------------------------------------------------------

    pub fn save_drivers(&self, items: &[DriverInfo]) -> StorageResult<()> {
        let mut conn = self.pool.get()?;
        repository::drivers::insert_many(&mut conn, items)
    }

    pub fn list_drivers(&self, session_id: Uuid, pagination: Pagination) -> StorageResult<Page<DriverInfo>> {
        let conn = self.pool.get()?;
        repository::drivers::list(&conn, session_id, pagination)
    }

    // ---- Modules --------------------------------------------------------

    pub fn save_modules(&self, items: &[ModuleInfo]) -> StorageResult<()> {
        let mut conn = self.pool.get()?;
        repository::modules::insert_many(&mut conn, items)
    }

    pub fn list_modules_for_process(&self, session_id: Uuid, pid: u32, pagination: Pagination) -> StorageResult<Page<ModuleInfo>> {
        let conn = self.pool.get()?;
        repository::modules::list_for_process(&conn, session_id, pid, pagination)
    }

    // ---- File activity ----------------------------------------------------

    pub fn save_file_activity(&self, items: &[FileActivity]) -> StorageResult<()> {
        let mut conn = self.pool.get()?;
        repository::file_activity::insert_many(&mut conn, items)
    }

    pub fn list_file_activity(&self, session_id: Uuid, pagination: Pagination) -> StorageResult<Page<FileActivity>> {
        let conn = self.pool.get()?;
        repository::file_activity::list(&conn, session_id, pagination)
    }

    // ---- Network activity ---------------------------------------------------

    pub fn save_network_activity(&self, items: &[NetworkActivity]) -> StorageResult<()> {
        let mut conn = self.pool.get()?;
        repository::network::insert_many(&mut conn, items)
    }

    pub fn list_network_activity(&self, session_id: Uuid, pagination: Pagination) -> StorageResult<Page<NetworkActivity>> {
        let conn = self.pool.get()?;
        repository::network::list(&conn, session_id, pagination)
    }

    // ---- Config entries -----------------------------------------------------

    pub fn save_config_entries(&self, items: &[ConfigEntry]) -> StorageResult<()> {
        let mut conn = self.pool.get()?;
        repository::config_entries::insert_many(&mut conn, items)
    }

    pub fn list_config_entries(&self, session_id: Uuid, pagination: Pagination) -> StorageResult<Page<ConfigEntry>> {
        let conn = self.pool.get()?;
        repository::config_entries::list(&conn, session_id, pagination)
    }

    // ---- Timeline -------------------------------------------------------

    pub fn save_timeline(&self, items: &[TimelineEntry]) -> StorageResult<()> {
        let mut conn = self.pool.get()?;
        repository::timeline::insert_many(&mut conn, items)
    }

    pub fn get_timeline(&self, session_id: Uuid) -> StorageResult<Vec<TimelineEntry>> {
        let conn = self.pool.get()?;
        repository::timeline::list_all(&conn, session_id)
    }

    // ---- Dependency graph -------------------------------------------------

    pub fn save_graph(&self, session_id: Uuid, graph: &DependencyGraph) -> StorageResult<()> {
        let mut conn = self.pool.get()?;
        repository::graph::insert_graph(&mut conn, session_id, graph)
    }

    pub fn get_graph(&self, session_id: Uuid) -> StorageResult<DependencyGraph> {
        let conn = self.pool.get()?;
        repository::graph::get_graph(&conn, session_id)
    }
}
