use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use spm_analysis::{HistoricalComparator, ReportGenerator};
use spm_core::ProcessRole;
use spm_storage::{Pagination, ProcessFilter};
use uuid::Uuid;

use crate::error::{bad_request, ApiError};
use crate::state::{AppState, CaptureStatus};
use crate::{platform_collectors, VERSION};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions/latest", get(latest_session))
        .route("/api/sessions/:id", get(get_session).delete(delete_session))
        .route("/api/sessions/:id/processes", get(list_processes))
        .route("/api/sessions/:id/processes/search", get(search_processes))
        .route("/api/sessions/:id/processes/:process_id", get(get_process))
        .route("/api/sessions/:id/processes/:process_id/modules", get(list_modules))
        .route("/api/sessions/:id/services", get(list_services))
        .route("/api/sessions/:id/drivers", get(list_drivers))
        .route("/api/sessions/:id/file-activity", get(list_file_activity))
        .route("/api/sessions/:id/network-activity", get(list_network_activity))
        .route("/api/sessions/:id/config-entries", get(list_config_entries))
        .route("/api/sessions/:id/timeline", get(get_timeline))
        .route("/api/sessions/:id/graph", get(get_graph))
        .route("/api/sessions/:id/report", get(get_report))
        .route("/api/compare", get(compare_sessions))
        .route("/api/capture", post(start_capture))
        .route("/api/capture/:id/status", get(capture_status))
        .route("/api/ws", get(ws_upgrade))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok", "version": VERSION }))
}

#[derive(Deserialize)]
struct PageParams {
    limit: Option<u32>,
    offset: Option<u32>,
}
impl PageParams {
    fn pagination(&self) -> Pagination {
        Pagination::new(self.limit.unwrap_or(0), self.offset.unwrap_or(0))
    }
}

async fn list_sessions(State(state): State<AppState>, Query(p): Query<PageParams>) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.storage.list_sessions(p.pagination())?))
}

async fn latest_session(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    match state.storage.latest_session()? {
        Some(session) => Ok(Json(session)),
        None => Err(bad_request("no sessions captured yet")),
    }
}

async fn get_session(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.storage.get_session(id)?))
}

async fn delete_session(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<impl IntoResponse, ApiError> {
    state.storage.delete_session(id)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct ProcessQueryParams {
    limit: Option<u32>,
    offset: Option<u32>,
    pid: Option<u32>,
    name: Option<String>,
    user: Option<String>,
    role: Option<String>,
    signed: Option<bool>,
}

async fn list_processes(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(p): Query<ProcessQueryParams>,
) -> Result<impl IntoResponse, ApiError> {
    let filter = ProcessFilter {
        pid: p.pid,
        name_contains: p.name.clone(),
        user: p.user.clone(),
        role: p.role.as_deref().and_then(parse_role),
        signed_only: p.signed,
    };
    let pagination = Pagination::new(p.limit.unwrap_or(0), p.offset.unwrap_or(0));
    Ok(Json(state.storage.list_processes(session_id, &filter, pagination)?))
}

#[derive(Deserialize)]
struct SearchParams {
    q: String,
    limit: Option<u32>,
    offset: Option<u32>,
}

async fn search_processes(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(p): Query<SearchParams>,
) -> Result<impl IntoResponse, ApiError> {
    let pagination = Pagination::new(p.limit.unwrap_or(0), p.offset.unwrap_or(0));
    Ok(Json(state.storage.search_processes(session_id, &p.q, pagination)?))
}

async fn get_process(
    State(state): State<AppState>,
    Path((_session_id, process_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.storage.get_process(process_id)?))
}

async fn list_modules(
    State(state): State<AppState>,
    Path((session_id, process_id)): Path<(Uuid, Uuid)>,
    Query(p): Query<PageParams>,
) -> Result<impl IntoResponse, ApiError> {
    let process = state.storage.get_process(process_id)?;
    Ok(Json(state.storage.list_modules_for_process(session_id, process.pid, p.pagination())?))
}

async fn list_services(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(p): Query<PageParams>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.storage.list_services(session_id, p.pagination())?))
}

async fn list_drivers(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(p): Query<PageParams>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.storage.list_drivers(session_id, p.pagination())?))
}

async fn list_file_activity(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(p): Query<PageParams>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.storage.list_file_activity(session_id, p.pagination())?))
}

async fn list_network_activity(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(p): Query<PageParams>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.storage.list_network_activity(session_id, p.pagination())?))
}

async fn list_config_entries(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(p): Query<PageParams>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.storage.list_config_entries(session_id, p.pagination())?))
}

async fn get_timeline(State(state): State<AppState>, Path(session_id): Path<Uuid>) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.storage.get_timeline(session_id)?))
}

async fn get_graph(State(state): State<AppState>, Path(session_id): Path<Uuid>) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.storage.get_graph(session_id)?))
}

#[derive(Deserialize)]
struct ReportParams {
    #[serde(default = "default_format")]
    format: String,
}
fn default_format() -> String {
    "json".to_string()
}

async fn get_report(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(p): Query<ReportParams>,
) -> Result<Response, ApiError> {
    let generator = ReportGenerator::new(&state.storage);
    let (content_type, body) = match p.format.as_str() {
        "json" => ("application/json", generator.to_json(session_id)?),
        "csv" => ("text/csv", generator.to_csv_processes(session_id)?),
        "markdown" | "md" => ("text/markdown", generator.to_markdown(session_id)?),
        "html" => ("text/html", generator.to_html(session_id)?),
        "sqlite" => {
            let path = std::env::temp_dir().join(format!("spm-export-{session_id}.db"));
            generator.export_sqlite(session_id, &path)?;
            let bytes = std::fs::read(&path).map_err(|e| anyhow::anyhow!(e))?;
            let _ = std::fs::remove_file(&path);
            return Ok((
                [
                    (header::CONTENT_TYPE, "application/vnd.sqlite3".to_string()),
                    (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{session_id}.db\"")),
                ],
                bytes,
            )
                .into_response());
        }
        other => return Err(bad_request(format!("unknown format '{other}' (expected json|csv|markdown|html|sqlite)"))),
    };
    Ok(([(header::CONTENT_TYPE, content_type)], body).into_response())
}

#[derive(Deserialize)]
struct CompareParams {
    baseline: Uuid,
    target: Uuid,
}

async fn compare_sessions(State(state): State<AppState>, Query(p): Query<CompareParams>) -> Result<impl IntoResponse, ApiError> {
    let comparison = HistoricalComparator::new(&state.storage).compare(p.baseline, p.target)?;
    Ok(Json(comparison))
}

#[derive(Deserialize, Default)]
struct CaptureRequest {
    notes: Option<String>,
    capture_window_secs: Option<u64>,
}

async fn start_capture(State(state): State<AppState>, body: Option<Json<CaptureRequest>>) -> Result<impl IntoResponse, ApiError> {
    let req = body.map(|Json(b)| b).unwrap_or_default();
    let session = spm_orchestrator::begin_session(&state.storage, req.notes).map_err(ApiError::from)?;
    state.set_status(session.id, CaptureStatus::Running);

    let capture_window = Duration::from_secs(req.capture_window_secs.unwrap_or(5));
    let session_id = session.id;
    let background_state = state.clone();
    tokio::spawn(async move {
        let snapshot = platform_collectors::snapshot_collectors();
        let streaming = platform_collectors::streaming_collectors();
        let result = spm_orchestrator::run_capture_for_session(
            &background_state.storage,
            &session,
            snapshot,
            streaming,
            capture_window,
        )
        .await;
        match result {
            Ok(()) => background_state.set_status(session_id, CaptureStatus::Complete),
            Err(e) => {
                tracing::error!(session_id = %session_id, error = %e, "capture failed");
                background_state.set_status(session_id, CaptureStatus::Failed { error: e.to_string() });
            }
        }
    });

    Ok((StatusCode::ACCEPTED, Json(json!({ "session_id": session_id, "status": "running" }))))
}

async fn capture_status(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<impl IntoResponse, ApiError> {
    match state.get_status(id) {
        Some(status) => Ok(Json(json!({ "session_id": id, "status": status }))),
        None => Err(bad_request("unknown capture id")),
    }
}

async fn ws_upgrade(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.events.subscribe();
    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(event) => {
                        let Ok(text) = serde_json::to_string(&event) else { continue };
                        if socket.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            incoming = socket.recv() => {
                if incoming.is_none() {
                    break;
                }
            }
        }
    }
}

fn parse_role(s: &str) -> Option<ProcessRole> {
    Some(match s {
        "kernel_process" => ProcessRole::KernelProcess,
        "system" => ProcessRole::System,
        "service" => ProcessRole::Service,
        "daemon" => ProcessRole::Daemon,
        "scheduled_task" => ProcessRole::ScheduledTask,
        "login_item" => ProcessRole::LoginItem,
        "user_application" => ProcessRole::UserApplication,
        "unknown" => ProcessRole::Unknown,
        _ => return None,
    })
}
