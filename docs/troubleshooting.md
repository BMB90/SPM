# Troubleshooting

## Build

**`error: linker `link.exe` not found`**
MSVC Build Tools aren't installed (or the C++ workload wasn't selected).
Install "Desktop development with C++" via the Visual Studio Installer,
or run:
```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools
& "C:\Program Files (x86)\Microsoft Visual Studio\Installer\setup.exe" modify `
  --installPath "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools" `
  --add Microsoft.VisualStudio.Workload.VCTools --quiet --norestart
```
(The bootstrapper's `--wait` flag only applies to the initial installer,
not `setup.exe modify` — omit it there or you'll get a "unknown option"
error.)

**`error: failed to select a version for `windows`... feature`**
A `spm-collector-windows`/`spm-desktop` `Cargo.toml` references a
`windows` crate feature that doesn't exist in the pinned version. Check
the exact feature name against the `windows` crate's docs for the
version in use (`Cargo.toml`'s `[target.'cfg(windows)'.dependencies.windows]`
block) — feature names changed across `windows` 0.52 → 0.58.

**Tauri build fails looking for icon files**
`apps/desktop/src-tauri/tauri.conf.json`'s `bundle.icon` list must point
at files that exist under `apps/desktop/src-tauri/icons/`. Regenerate
them with `npx @tauri-apps/cli icon <source.png>` from that directory, or
see `docs/deployment.md`.

## Capture

**A collector never returns data / "collector unavailable, skipping" in logs**
Check `is_available()` for that collector — several require Administrator
(`windows.etw_process_trace`) or an optional component
(`windows.sysmon` needs Sysmon installed). This is by design: the capture
still completes with the collectors that *are* available. Re-run elevated
for the most complete capture.

**Capture is slow**
`ProcessSnapshotCollector` hashes (SHA-256) and Authenticode-checks every
process's executable by default — on a machine with many running
processes this dominates capture time. Pass `--no-enrich` to `spm capture`
to skip it (`POST /api/capture` doesn't currently expose this flag — see
`spm-api/src/routes.rs`'s `start_capture` handler for where to add it).

**File Activity / Network Activity pages are always empty**
Expected on Windows today — those collectors are a documented gap, not a
bug. See `docs/collector-architecture.md`'s "Not yet implemented" note.

## API / Desktop UI

**Desktop UI shows "No sessions" / network errors in the console**
`spm-api` isn't running, or is running against a different
`SPM_DB_PATH` / port than the UI's `VITE_SPM_API_URL` expects. Start
`cargo run -p spm-api` and confirm `http://127.0.0.1:7878/api/health`
responds before opening the UI.

**CORS errors in the browser console**
`spm-api` enables `CorsLayer::permissive()` unconditionally
(`spm-api/src/main.rs`) — if you still see CORS errors, you're likely
hitting a different port than the one the UI is configured for
(`VITE_SPM_API_URL`), not an actual CORS policy rejection.

**`npm install` warns about esbuild's postinstall script being blocked**
npm's `allow-scripts` supply-chain guard blocks new packages' install
scripts by default. Run `npm approve-scripts esbuild` (or the specific
package npm names in the warning) and re-run `npm install`/`npm rebuild`.

## Database

**`database is locked` errors under concurrent access**
The connection pool opens SQLite with `PRAGMA journal_mode = WAL`
(`spm-storage/src/db.rs`), which allows one writer + multiple readers
concurrently — a lock error usually means an external tool (e.g. a
`sqlite3` CLI session) has the file open with an incompatible journal
mode, or a very long-running transaction is still open. Close other
connections and retry.

**Migrations didn't apply / schema looks stale**
`Storage::open`/`open_in_memory` run `apply_migrations` synchronously on
first connect — if you see a schema mismatch, check
`schema_migrations`'s `MAX(version)` in the database against
`MIGRATIONS`'s last entry in `crates/spm-storage/src/migrations.rs`; a
partially-applied migration (crash mid-transaction) would roll back
automatically since each migration runs in its own transaction, so this
should be rare.
