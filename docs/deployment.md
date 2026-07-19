# Deployment

SPM is a local diagnostics tool, not a hosted service — "deployment"
here means "getting the binaries and (optionally) the desktop app onto a
machine you want to analyze."

## Building release binaries

```powershell
cargo build --release --workspace
# Binaries land in target/release/:
#   spm.exe        (spm-cli)
#   spm-api.exe
#   spm-desktop.exe (if apps/desktop/src-tauri is included in the workspace build)
```

`Cargo.toml`'s `[profile.release]` enables `lto = true` and `strip = true`
— release binaries are optimized and stripped of debug symbols. Expect a
noticeably longer link step than `cargo build` (dev profile) for that
reason.

## Headless (CLI + API) deployment

For servers or any machine where you don't want a GUI:

1. Copy `spm.exe` (or the Linux `spm` binary, once
   `spm-collector-linux` is implemented) and `spm-api.exe` to the
   target machine.
2. `spm.exe capture` to take a snapshot (writes to `./data/spm.db` by
   default; set a working directory or pass `--db <path>` — see
   `spm-cli --help`).
3. Optionally run `spm-api.exe` (set `SPM_DB_PATH`/`SPM_API_PORT` as
   needed) and point any HTTP client — including a browser hitting the
   built desktop UI's static assets served some other way — at it.

Both binaries are self-contained (SQLite is compiled in via `rusqlite`'s
`bundled` feature) — no separate SQLite install, no .NET/WebView2
requirement (that's only needed for the desktop shell).

## Desktop app packaging

```powershell
cd apps/desktop
npm install
npm run tauri build
```

This produces a platform-native installer (MSI/NSIS on Windows) under
`apps/desktop/src-tauri/target/release/bundle/`. The bundle icon set
lives in `apps/desktop/src-tauri/icons/` — replace the placeholder
icons generated for this scaffold with real artwork before shipping (any
image editor, or `npx @tauri-apps/cli icon <source.png>` to regenerate
the full set from one source image).

The desktop app expects `spm-api` reachable at
`VITE_SPM_API_URL` (baked in at frontend build time, default
`http://127.0.0.1:7878`). For a fully self-contained desktop
distribution, the natural next step is making `src-tauri`'s `main.rs`
spawn `spm-api` as a sidecar process on startup (Tauri's
[sidecar](https://v2.tauri.app/develop/sidecar/) mechanism) — this
scaffold intentionally keeps them separate processes for simplicity; see
`apps/desktop/src-tauri/src/main.rs`'s doc comment for the seam.

## Windows privilege requirements

Most collectors work unelevated. Two need Administrator:

- `EtwProcessTraceCollector` (real-time process tracing) —
  `is_available()` returns `Unavailable` and the engine skips it
  cleanly when not elevated; the rest of the capture still completes.
- Reading some protected registry/Task Scheduler paths and Toolhelp
  module snapshots of other users' processes — these fail per-item
  (not per-collector) and are silently skipped, so an unelevated capture
  is still useful, just less complete.

For the most complete capture, run `spm.exe capture` (or `spm-api.exe`)
from an elevated (Run as Administrator) prompt.

## Data retention

Every capture is a new session row in the SQLite database; nothing is
deleted automatically. `spm.exe sessions` lists them;
`DELETE /api/sessions/:id` (or the equivalent future CLI command) removes
one and everything it captured via `ON DELETE CASCADE`. For long-running
deployments, prune old sessions periodically or point `SPM_DB_PATH` at a
rotated location.
