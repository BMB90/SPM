# Collector Architecture

## The two traits

```rust
#[async_trait]
pub trait Collector: Send + Sync {
    fn id(&self) -> &'static str;
    fn platform(&self) -> Platform;
    fn category(&self) -> CollectorCategory;
    fn description(&self) -> &'static str;
    fn is_available(&self) -> SpmResult<()> { Ok(()) }
    async fn collect(&self, ctx: &CollectorContext) -> SpmResult<Vec<Event>>;
}

#[async_trait]
pub trait StreamingCollector: Send + Sync {
    // same id/platform/category/description/is_available
    async fn stream(&self, ctx: &CollectorContext, tx: mpsc::Sender<Event>) -> SpmResult<()>;
}
```

(`crates/spm-core/src/collector.rs`)

- **`Collector`** for anything that answers "what's true right now" —
  enumerate and return. Registry keys, WMI queries, `Toolhelp32Snapshot`,
  `/proc` scans.
- **`StreamingCollector`** for anything that observes events *as they
  happen* over the capture window — ETW, eBPF, audit subsystem, journal
  tailing. `SessionManager::capture` drains these via an `mpsc` channel
  until `ctx.capture_window` elapses or the collector's `stream()`
  returns.

`is_available()` lets a collector opt out cleanly (missing privilege,
absent component like Sysmon) — the engine logs why and skips it rather
than failing the whole capture. Every Windows collector that needs
elevation or an optional component (`EtwProcessTraceCollector`,
`SysmonCollector`) implements this.

## Windows collectors (`spm-collector-windows`)

| Collector | Source | Notes |
|---|---|---|
| `ProcessSnapshotCollector` | `sysinfo` (Toolhelp/NtQuerySystemInformation/PDH under the hood) | Enriches with SHA-256 (`hash.rs`) + Authenticode status (`signature.rs`) on the blocking pool |
| `StartupRegistryCollector` | Registry `Run`/`RunOnce` (HKLM/HKCU × native/Wow6432Node) + both Startup folders | `winreg` |
| `ServiceCollector` | WMI `Win32_Service` + registry `DependOnService`/`DelayedAutostart` | `wmi` crate |
| `DriverCollector` | WMI `Win32_SystemDriver` | Load order is enumeration-order, not true NT load order (see doc comment) |
| `ModuleCollector` | `CreateToolhelp32Snapshot(TH32CS_SNAPMODULE)` per process | Access-denied on protected processes is expected/skipped |
| `ScheduledTaskCollector` | `%WINDIR%\System32\Tasks\**` XML files | Avoids `ITaskService` COM/IDispatch marshalling — see file header comment for the tradeoff |
| `EventLogCollector` | `Evt*` (wevtapi) query against the `System` channel | Filters to boot/service-lifecycle event IDs (7036, 6005/6009/6013, 12/13, ...) |
| `SysmonCollector` | Same `Evt*` machinery against `Microsoft-Windows-Sysmon/Operational` | `is_available()` checks the Sysmon service registry key first |
| `EtwProcessTraceCollector` (streaming) | ETW `Microsoft-Windows-Kernel-Process` provider via `ferrisetw` | Requires Administrator — `is_available()` checks `TokenElevation` |

Not yet implemented (documented gaps, not silently faked): per-process
file I/O (`FileActivity`) and per-process network connections
(`NetworkActivity`) — both would come from Sysmon/ETW file-IO and
network-provider events respectively, using the same `Evt*`/ETW machinery
already in this crate. The UI's File Activity and Network pages show an
explicit empty-state note rather than fabricated rows.

## Linux collectors (`spm-collector-linux`)

**Interface-only stubs.** Every collector implements the real trait and
returns `is_available() -> Err(Unavailable)`, so the engine, storage,
analysis, and API layers all work unmodified on Linux — nothing downstream
needed to change to add this crate. See the crate's module doc comment
for the planned real implementation of each (`/proc`, systemd D-Bus,
`/proc/modules`, netlink/udev, audit subsystem, eBPF tracepoints via
`aya`/`libbpf-rs`, `sd-journal`).

## Why collectors don't call each other

`ProcessSnapshotCollector` doesn't know about `StartupRegistryCollector`,
and neither knows about `ServiceCollector` — each just emits raw
observations. Correlating "this process's path matches that registry
value" happens once, downstream, in
`spm-analysis::StartupSourceDetector`, operating on the merged
`NormalizedSession`. This keeps collectors simple, parallel-safe, and
independently testable, and means adding a new correlation rule never
requires touching a collector.

## Testing without real OS access

`crates/spm-engine/tests/capture_integration.rs` defines synthetic
`Collector`/`StreamingCollector` implementations that emit a canned,
deterministic boot sequence — this is what the spec calls "mock
collectors" / "synthetic boot event generators." Use the same pattern to
test new collectors' *consumers* (analysis rules, storage round-trips)
without needing the real Windows/Linux instrumentation available in CI.
