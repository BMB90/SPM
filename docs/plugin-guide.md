# Plugin Guide: Adding a New Collector

Worked example: adding a real `/proc`-based process collector to
`spm-collector-linux`, replacing the `ProcessCollector` stub.

## 1. Implement the trait

```rust
// crates/spm-collector-linux/src/process.rs
use async_trait::async_trait;
use spm_core::{Collector, CollectorCategory, CollectorContext, Event, EventPayload, Platform, ProcessInfo, SpmResult};

pub struct ProcessCollector;

#[async_trait]
impl Collector for ProcessCollector {
    fn id(&self) -> &'static str { "linux.process_snapshot" }
    fn platform(&self) -> Platform { Platform::Linux }
    fn category(&self) -> CollectorCategory { CollectorCategory::Process }
    fn description(&self) -> &'static str { "procfs-based process enumeration" }

    async fn collect(&self, ctx: &CollectorContext) -> SpmResult<Vec<Event>> {
        let session_id = ctx.session.id;
        let events = tokio::task::spawn_blocking(move || scan_proc(session_id))
            .await
            .map_err(|e| spm_core::SpmError::Collector {
                collector: "linux.process_snapshot".to_string(),
                message: e.to_string(),
            })?;
        Ok(events)
    }
}

fn scan_proc(session_id: uuid::Uuid) -> Vec<Event> {
    // Walk /proc/<pid>/{stat,status,cmdline,environ,exe,cwd}, build
    // ProcessInfo per pid, wrap each in Event::new(session_id, "linux.process_snapshot",
    // EventPayload::ProcessStarted(Box::new(info))). Mirror
    // spm-collector-windows/src/process.rs's structure — same target type,
    // different source.
    Vec::new()
}
```

Rules that keep this pluggable:

- **Only emit `spm_core` types.** Never invent a parallel model — if the
  data doesn't fit an existing field, extend the struct in `spm-core`
  (see `docs/developer-guide.md`'s "adding a field" section) rather than
  reaching for `EventPayload::Raw` for anything long-term.
- **Fail soft.** Return `Ok(vec![])` or skip individual entries on
  permission errors (`/proc/<pid>` disappearing mid-scan, protected PIDs)
  — never `panic!` or bubble a single bad process into aborting the whole
  collector.
- **Do blocking I/O on `spawn_blocking`.** `collect`/`stream` run on the
  async runtime; filesystem/registry/WMI/COM calls are all synchronous
  and should move to the blocking pool (every existing collector follows
  this pattern — copy it).

## 2. Register it

```rust
// crates/spm-collector-linux/src/lib.rs
pub fn all_snapshot_collectors() -> Vec<Arc<dyn Collector>> {
    vec![
        Arc::new(ProcessCollector),   // now real, not the macro-generated stub
        Arc::new(SystemdCollector),
        // ...
    ]
}
```

Nothing in `spm-cli`, `spm-api`, `spm-engine`, `spm-storage`, or
`spm-analysis` needs to change — they all consume
`all_snapshot_collectors()`/`all_streaming_collectors()` through the
`Collector`/`StreamingCollector` trait objects.

## 3. Test it

Add a unit test for any pure-logic helper (parsing `/proc/<pid>/stat`
fields, for instance) directly in the collector module — see
`spm-collector-windows/src/eventlog.rs`'s `parse_event_xml` tests for the
pattern (feed it a fixed sample string, assert the parsed struct).

For testing how the rest of the pipeline reacts to this collector's
output, don't try to run it for real in CI — write a synthetic collector
implementing the same trait with canned data instead, following
`crates/spm-engine/tests/capture_integration.rs`.

## 4. Document it

Add a row to the collector table in `docs/collector-architecture.md` and,
if it's the first real implementation of a previously-documented gap
(like this `/proc` example replacing a stub), remove it from the "not yet
implemented" list.

## Adding an entirely new OS

Same shape, one crate up: create `spm-collector-<os>`, depend only on
`spm-core` (plus that OS's native bindings), implement
`all_snapshot_collectors()`/`all_streaming_collectors()`, and add one more
`#[cfg(target_os = "...")]` arm to the `platform_collectors` module in
both `spm-cli` and `spm-api`. That's the entire integration surface.
