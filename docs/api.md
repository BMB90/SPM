# REST + WebSocket API

Base URL: `http://127.0.0.1:7878` (configurable via `SPM_API_PORT`).
All responses are JSON. Errors are `{"error": "message"}` with an
appropriate 4xx/5xx status.

## Sessions

| Method | Path | Description |
|---|---|---|
| GET | `/api/health` | `{status, version}` |
| GET | `/api/sessions?limit=&offset=` | Paginated session list, newest first |
| GET | `/api/sessions/latest` | Most recent session |
| GET | `/api/sessions/:id` | One session |
| DELETE | `/api/sessions/:id` | Delete a session and all its data (cascades) |

## Processes

| Method | Path | Description |
|---|---|---|
| GET | `/api/sessions/:id/processes?limit=&offset=&pid=&name=&user=&role=&signed=` | Filtered, paginated |
| GET | `/api/sessions/:id/processes/search?q=&limit=&offset=` | Substring search across name/path/command line/sha256/pid |
| GET | `/api/sessions/:id/processes/:process_id` | One process by its row id (not PID) |
| GET | `/api/sessions/:id/processes/:process_id/modules?limit=&offset=` | Loaded modules for that process |

`role` accepts: `kernel_process`, `system`, `service`, `daemon`,
`scheduled_task`, `login_item`, `user_application`, `unknown`.

## Other entities

| Method | Path |
|---|---|
| GET | `/api/sessions/:id/services?limit=&offset=` |
| GET | `/api/sessions/:id/drivers?limit=&offset=` |
| GET | `/api/sessions/:id/file-activity?limit=&offset=` |
| GET | `/api/sessions/:id/network-activity?limit=&offset=` |
| GET | `/api/sessions/:id/config-entries?limit=&offset=` |
| GET | `/api/sessions/:id/timeline` | Full timeline, not paginated (see below) |
| GET | `/api/sessions/:id/graph` | Full dependency graph `{nodes, edges}` |

Paginated endpoints return `{items, total, limit, offset}`.
`/timeline` and `/graph` return the whole thing unpaginated — the UI needs
the complete picture to zoom/filter/lay out client-side.

## Reports

`GET /api/sessions/:id/report?format=json|csv|markdown|html|sqlite`

- `json` — the full `SessionReport` structure (session + every entity +
  timeline + graph) as pretty-printed JSON.
- `csv` — process table only (the entity with the most useful flat
  columns).
- `markdown` / `html` — a human-readable summary (top security findings,
  boot-stage counts, first 200 timeline rows).
- `sqlite` — a standalone SQLite file containing this session's data,
  downloaded as an attachment. Openable with any `spm-storage`-based tool.

## Comparison

`GET /api/compare?baseline=<uuid>&target=<uuid>` → `SessionComparison`:
added/removed processes, added/removed startup items, executable path
drift, and boot-duration delta between two sessions.

## Capture control

| Method | Path | Description |
|---|---|---|
| POST | `/api/capture` | Body: `{notes?, capture_window_secs?}`. Returns `202 {session_id, status: "running"}` immediately; capture runs in the background. |
| GET | `/api/capture/:id/status` | `{session_id, status}` where status is `running`, `complete`, or `failed` (+ `error`) |
| GET | `/api/ws` | WebSocket; server pushes `{session_id, status}` on every capture status change |

The desktop UI's "New Capture" button uses `POST /api/capture` and polls
`/api/capture/:id/status` (a future iteration could subscribe to `/api/ws`
instead — the event shape is identical).

## Machine-readable schema

`GET /api/sessions/:id/report?format=json` is the authoritative schema for
every entity — it's the same `SessionReport` struct
(`crates/spm-analysis/src/reporting.rs`) serialized directly, with no
lossy transformation. The TypeScript types in
`apps/desktop/src/api/types.ts` mirror it field-for-field; keep both in
sync when changing `spm-core` structs (see `docs/developer-guide.md`).
