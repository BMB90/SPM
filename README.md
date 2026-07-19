# SPM — Startup Intelligence Platform

SPM reconstructs the complete startup chain of a machine — from kernel/init
through service startup, driver loading, autostart applications, and user
login, to an idle desktop — and answers *what started, when, why, and at what
cost*.

## Architecture

```
crates/
  spm-core                 Cross-platform domain model, event model, Collector trait
  spm-engine                Core engine: session mgmt, normalization, dependency graph, timeline
  spm-storage                SQLite persistence + repository abstraction
  spm-collector-windows     Windows collectors (registry, SCM, WMI, ETW, Event Log, Task Scheduler, Authenticode, hashing)
  spm-collector-linux       Linux collector (interface-only stub — see crates/spm-collector-linux/README.md)
  spm-analysis               Startup-source detection, dependency reconstruction, suspicious-activity heuristics
  spm-api                    REST + WebSocket API (axum)
  spm-cli                    Headless capture / query / report CLI
apps/
  desktop                    Tauri + React + TypeScript desktop UI
docs/                        Architecture, developer, API, schema, and operational documentation
```

See [docs/architecture.md](docs/architecture.md) for the full design and
[docs/developer-guide.md](docs/developer-guide.md) to get building.

## Status

Actively developed. Windows collectors are implemented against real OS
instrumentation (registry, SCM/WMI, ETW, Windows Event Log, Task Scheduler,
Authenticode, performance counters). Linux collectors are defined behind the
same `Collector` trait but are currently interface-only stubs pending
implementation and testing on Linux hardware.

## Quick start

```powershell
# Build everything
cargo build --workspace

# Capture the current boot/startup state into a new session
cargo run -p spm-cli -- capture

# Start the REST API (serves the most recent session by default)
cargo run -p spm-api

# Run the desktop UI (requires Node.js and the Tauri CLI)
cd apps/desktop
npm install
npm run tauri dev
```

## License

MIT — see [LICENSE](LICENSE).
