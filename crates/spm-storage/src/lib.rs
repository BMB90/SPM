//! SQLite persistence layer. `Storage` is the public entry point; the
//! `repository` module holds one file per domain entity mapping
//! `spm_core` types to/from SQL rows. Complex nested fields (arguments,
//! environment, evidence lists, attribute maps) are stored as JSON text
//! columns — scalar fields used for filtering/sorting (pid, name, path,
//! state, timestamps) are real columns with indexes.
//!
//! Swapping SQLite for PostgreSQL later means adding a second `db`/`repository`
//! implementation behind the same `Storage` API; nothing outside this
//! crate should need to change.

pub mod db;
pub mod error;
pub mod migrations;
pub mod pagination;
pub mod repository;
pub mod storage;

pub use error::{StorageError, StorageResult};
pub use pagination::{Page, Pagination};
pub use storage::{ProcessFilter, Storage};

#[cfg(test)]
mod tests {
    use super::*;
    use spm_core::{BootSession, Platform, ProcessInfo};

    #[test]
    fn round_trips_a_session_and_process() {
        let storage = Storage::open_in_memory().expect("open in-memory db");

        let session = BootSession::new("test-host", Platform::Windows, "Windows 11 Pro 24H2");
        storage.create_session(&session).unwrap();

        let fetched = storage.get_session(session.id).unwrap();
        assert_eq!(fetched.hostname, "test-host");
        assert_eq!(fetched.platform, Platform::Windows);

        let mut process = ProcessInfo::new(session.id, 4242, "explorer.exe");
        process.ppid = Some(4);
        process.executable_path = Some(r"C:\Windows\explorer.exe".to_string());
        storage.save_processes(&[process.clone()]).unwrap();

        let page = storage
            .list_processes(session.id, &ProcessFilter::default(), Pagination::default())
            .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].pid, 4242);
        assert_eq!(page.items[0].executable_path.as_deref(), Some(r"C:\Windows\explorer.exe"));

        let found = storage.search_processes(session.id, "explorer", Pagination::default()).unwrap();
        assert_eq!(found.total, 1);

        let not_found = storage.search_processes(session.id, "chrome", Pagination::default()).unwrap();
        assert_eq!(not_found.total, 0);
    }

    #[test]
    fn list_sessions_orders_newest_first() {
        let storage = Storage::open_in_memory().unwrap();
        let s1 = BootSession::new("host", Platform::Windows, "11");
        std::thread::sleep(std::time::Duration::from_millis(5));
        let s2 = BootSession::new("host", Platform::Windows, "11");
        storage.create_session(&s1).unwrap();
        storage.create_session(&s2).unwrap();

        let page = storage.list_sessions(Pagination::default()).unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(page.items[0].id, s2.id);
    }
}
