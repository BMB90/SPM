# Developer Guide

## Prerequisites

- Rust (stable, MSVC toolchain on Windows) — `rustup show` should list
  `stable-x86_64-pc-windows-msvc` on Windows, or your platform's default
  on Linux.
- On Windows: MSVC Build Tools with the "Desktop development with C++"
  workload (needed for the MSVC linker `link.exe` and the Windows SDK).
- Node.js 20+ and npm, for the desktop UI.
- SQLite needs no separate install — `rusqlite`'s `bundled` feature
  compiles it from source as part of the Rust build.

## Building

```powershell
# Everything (all crates + the Tauri shell)
cargo build --workspace

# Just the backend crates (fast inner loop)
cargo build -p spm-core -p spm-engine -p spm-storage -p spm-analysis

# Windows collectors (only compiles on Windows — cfg-gated)
cargo build -p spm-collector-windows
```

## Running

```powershell
# Headless capture -> ./data/spm.db
cargo run -p spm-cli -- capture

# List captured sessions
cargo run -p spm-cli -- sessions

# Export a report
cargo run -p spm-cli -- report --format markdown --out report.md

# Start the REST API (defaults to http://127.0.0.1:7878, ./data/spm.db)
cargo run -p spm-api

# Desktop UI (needs spm-api running separately, or use `tauri dev` which
# only starts the frontend dev server — you still need `spm-api` running)
cd apps/desktop
npm install
npm run tauri dev
```

Environment variables:

| Variable | Used by | Default |
|---|---|---|
| `SPM_DB_PATH` | `spm-api` | `./data/spm.db` |
| `SPM_API_PORT` | `spm-api` | `7878` |
| `VITE_SPM_API_URL` | desktop UI (build-time) | `http://127.0.0.1:7878` |
| `RUST_LOG` | all Rust binaries (via `tracing-subscriber`'s `EnvFilter`) | info-level default |

## Testing

```powershell
cargo test --workspace
```

- Unit tests live next to the code they test (`#[cfg(test)] mod tests`).
- Integration tests live in each crate's `tests/` directory —
  `crates/spm-engine/tests/capture_integration.rs` is the main one: it
  exercises the full collect → normalize → analyze → graph → timeline
  pipeline using synthetic (mock) collectors instead of real OS
  instrumentation, so it runs identically on any machine/CI.
- Frontend: `cd apps/desktop && npx tsc -b` typechecks; there is no
  frontend test runner configured yet (see `docs/contributing.md` for
  how to add one).

## Project layout

See `docs/architecture.md` for the full crate map and data-flow diagram.
The short version: `spm-core` has the shared types, platform crates
(`spm-collector-*`) turn OS instrumentation into those types,
`spm-engine`/`spm-storage`/`spm-analysis` process and persist them, and
`spm-cli`/`spm-api` are the two entry points that wire it all together.

## Adding a new field to an existing entity

1. Add the field to the struct in `crates/spm-core/src/<entity>.rs`.
2. Add the column in `crates/spm-storage/src/migrations.rs` as a **new**
   migration (never edit a shipped one) plus the corresponding
   read/write code in `crates/spm-storage/src/repository/<entity>.rs`.
3. Populate it from the relevant collector(s) in
   `spm-collector-windows`/`spm-collector-linux`.
4. Add it to the TypeScript type in `apps/desktop/src/api/types.ts` and
   surface it in the relevant page/table if it's user-facing.
5. Update `docs/database-schema.md` and `docs/api.md`.

## Adding a new collector

See `docs/plugin-guide.md` — it's a step-by-step walkthrough using a real
example (adding a Linux `/proc`-based process collector).

## Code style

- `rustfmt`/`clippy` defaults; no repo-specific overrides.
- Prefer returning `Option`/`Result` over sentinel values; collectors
  should never panic on missing/malformed OS data — log and skip.
- Doc comments explain *why* a field/function exists when it's
  non-obvious (see `SecurityInfo::findings`, `StartupSource::evidence`);
  they don't restate the type signature.
