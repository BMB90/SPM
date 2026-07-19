# Architecture

SPM reconstructs a machine's complete startup chain — kernel init through
idle desktop — and answers *what started, when, why, and at what cost*.
This document explains how the pieces fit together and why they're split
the way they are.

## Design goals

1. **Platform-specific collection, platform-agnostic everything else.**
   Only two crates (`spm-collector-windows`, `spm-collector-linux`) know
   about ETW, WMI, the registry, systemd, or eBPF. Everything downstream —
   normalization, storage, analysis, the API, the UI — works against one
   shared vocabulary (`spm-core`) and never branches on OS.
2. **Evidence, not assertions.** Every "why did this start" answer
   (`StartupSource`) carries the literal registry value, unit file path,
   or parent PID that justified it, plus a confidence score. Nothing is
   classified by guesswork presented as fact.
3. **Collectors fail independently.** A collector that can't run (no
   Sysmon installed, not elevated, WMI unavailable) is skipped and logged
   — it never aborts the rest of the capture.
4. **The event model is the only seam.** New OS support or a new event
   source is "implement `Collector`/`StreamingCollector`, emit
   `spm_core::Event`s" — no changes to `spm-engine`, `spm-storage`,
   `spm-analysis`, or the API.

## Crate map

```
crates/
  spm-core            Domain model, Event envelope, Collector trait — zero I/O, zero platform code
  spm-engine          Session orchestration, event normalization, dependency graph, timeline
  spm-storage         SQLite schema + repository layer (Storage facade)
  spm-collector-windows   ETW, WMI, SCM, registry, Event Log, Task Scheduler, Authenticode, hashing
  spm-collector-linux     Interface-only stubs (see collector-architecture.md)
  spm-analysis        Startup-source detection, security heuristics, historical comparison, reporting
  spm-orchestrator    Glues engine+analysis+storage into one `run_capture()`, platform-agnostic
  spm-api             REST + WebSocket server (axum)
  spm-cli             Headless capture/query/report tool
apps/
  desktop             Tauri + React + TypeScript UI (talks to spm-api over HTTP)
```

Dependency direction is strictly one-way:

```
spm-core
  ↑
spm-engine, spm-collector-windows, spm-collector-linux
  ↑
spm-storage, spm-analysis
  ↑
spm-orchestrator
  ↑
spm-cli, spm-api
  ↑
apps/desktop (HTTP only — no Rust dependency)
```

## The capture pipeline

```
 ┌──────────────────┐   ┌──────────────────┐
 │ snapshot          │   │ streaming         │
 │ collectors        │   │ collectors (ETW)  │
 │ (registry, WMI,   │   │ run for the       │
 │ SCM, processes...) │   │ capture window    │
 └─────────┬─────────┘   └─────────┬─────────┘
           │  Vec<Event>            │  Vec<Event>
           └───────────┬────────────┘
                        ▼
              SessionManager::capture
              (spm-engine — runs collectors
               concurrently, merges output)
                        ▼
                 EventProcessor
         (dedups/merges by PID or key into
          one NormalizedSession)
                        ▼
        ┌───────────────┴────────────────┐
        ▼                                 ▼
StartupSourceDetector            SecurityAnalyzer
(spm-analysis — cross-refs        (spm-analysis — unsigned-in-
 processes against config          temp, LOLBin-spawned-by-Office,
 entries/services)                 kernel-process anomalies)
        └───────────────┬────────────────┘
                        ▼
              DependencyGraphBuilder
           (spm-engine — parent/child,
            service/driver deps, critical path)
                        ▼
                 TimelineBuilder
          (spm-engine — offsets, parallel
           groups, critical-path marking)
                        ▼
                    Storage
         (spm-storage — persists every
          entity + the graph + timeline)
```

`spm-orchestrator::run_capture` (and its split `begin_session` /
`run_capture_for_session` for the API's "report a session id immediately,
finish in the background" flow) is the only place that wires this whole
sequence together — see its doc comments for the exact call order.

## The event model (`spm_core::Event`)

Every collector — regardless of OS or instrumentation source — emits
`spm_core::Event { id, session_id, collector_id, observed_at, payload }`
where `payload` is one variant of `EventPayload`:
`ProcessStarted`/`ProcessUpdated`/`ProcessExited`, `ServiceObserved`,
`ServiceStateChanged`, `DriverObserved`, `ModuleLoaded`,
`FileActivityObserved`, `NetworkActivityObserved`, `ConfigEntryObserved`,
`BootStageReached`, and an escape hatch (`Raw`) for anything that doesn't
have a first-class model yet. See `crates/spm-core/src/event.rs`.

## Two collector shapes

- **`Collector`** (snapshot): runs once, returns a finite `Vec<Event>`.
  Registry enumeration, WMI queries, process snapshots — anything that
  answers "what's true right now."
- **`StreamingCollector`**: runs for `CollectorContext::capture_window`,
  pushing events onto an `mpsc::Sender<Event>` as they happen. ETW/eBPF
  process tracing — anything that answers "what happened during this
  window," including processes that started *and exited* before a
  snapshot collector would have seen them.

`SessionManager::capture` runs every snapshot collector concurrently via a
`JoinSet`, then drains every streaming collector until the capture window
elapses or they finish early, merging both into one event stream before
handing it to `EventProcessor`.

## Storage

SQLite via `rusqlite` + `r2d2` (connection pool), one table per entity,
JSON columns for nested/variable-shape fields (arguments, environment,
evidence lists, attribute maps), real columns + indexes for anything
that's filtered or sorted on. See `docs/database-schema.md`.

## Why Rust + Tauri/React

- Rust: the collectors do unsafe FFI (WinAPI, COM, ETW) and need to run
  with minimal overhead during boot-adjacent capture windows — a
  memory-safety bug in a collector shouldn't crash or corrupt the whole
  capture.
- Tauri: a native window with near-zero footprint hosting a normal web
  frontend, rather than shipping a full Chromium (Electron). The
  frontend is a thin HTTP client of `spm-api` — it could be replaced or
  run standalone in a browser without touching the Rust side.

## Extensibility

Adding a third OS means: create `spm-collector-<os>`, implement
`Collector`/`StreamingCollector` for its native instrumentation, add
`all_snapshot_collectors()`/`all_streaming_collectors()` functions, and
wire a `platform_collectors` cfg arm in `spm-cli`/`spm-api`. Nothing in
`spm-core`, `spm-engine`, `spm-storage`, or `spm-analysis` changes — see
`docs/plugin-guide.md` for the walkthrough.
