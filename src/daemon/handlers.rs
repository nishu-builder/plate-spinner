use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::{HookEvent, SessionStatus};
use super::state::{AppState, WsMessage};

#[derive(Serialize)]
pub struct StatusResponse {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key_configured: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hooks_installed: Option<bool>,
}

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

pub async fn status() -> Json<StatusResponse> {
    let api_key_configured = std::env::var("ANTHROPIC_API_KEY").is_ok();
    Json(StatusResponse {
        status: "ok".to_string(),
        api_key_configured: Some(api_key_configured),
        hooks_installed: Some(true),
    })
}

fn determine_status(event: &HookEvent) -> SessionStatus {
    if event.event_type == "stop" {
        return if event.error.is_some() {
            SessionStatus::Error
        } else {
            SessionStatus::Idle
        };
    }
    if event.event_type == "session_start" || event.event_type == "tool_start" {
        return SessionStatus::Running;
    }
    SessionStatus::from_tool(event.tool_name.as_deref().unwrap_or(""))
}

pub async fn post_event(
    State(state): State<Arc<AppState>>,
    Json(event): Json<HookEvent>,
) -> Json<serde_json::Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let status = determine_status(&event);

    {
        let db = state.db.lock().unwrap();
        let _ = db.upsert_session(
            &event.session_id,
            &event.project_path,
            event.transcript_path.as_deref(),
            event.git_branch.as_deref(),
            status.as_str(),
            &event.event_type,
            event.tool_name.as_deref(),
            &now,
        );

        if event.tool_name.as_deref() == Some("TodoWrite") {
            if let Some(params) = &event.tool_params {
                if let Some(todos) = params.get("todos") {
                    let _ = db.upsert_todos(&event.session_id, &todos.to_string(), &now);
                }
            }
        }

        let _ = db.insert_event(
            &event.session_id,
            &event.event_type,
            &serde_json::to_string(&event).unwrap_or_default(),
            &now,
        );
    }

    let _ = state.tx.send(WsMessage::SessionUpdate(event.session_id.clone()));
    Json(serde_json::json!({"status": "ok"}))
}

pub async fn get_sessions(State(state): State<Arc<AppState>>) -> Json<Vec<crate::models::Session>> {
    let db = state.db.lock().unwrap();
    Json(db.get_sessions().unwrap_or_default())
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    project_path: String,
}

pub async fn register_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Json<serde_json::Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let placeholder_id = {
        let db = state.db.lock().unwrap();
        db.register_placeholder(&req.project_path, &now).unwrap_or_default()
    };
    let _ = state.tx.send(WsMessage::SessionUpdate(placeholder_id.clone()));
    Json(serde_json::json!({"status": "ok", "placeholder_id": placeholder_id}))
}

pub async fn mark_stopped(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Json<serde_json::Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let session_ids = {
        let db = state.db.lock().unwrap();
        db.mark_stopped(&req.project_path, &now).unwrap_or_default()
    };
    for session_id in &session_ids {
        let _ = state.tx.send(WsMessage::SessionUpdate(session_id.clone()));
    }
    Json(serde_json::json!({"status": "ok", "count": session_ids.len()}))
}

pub async fn delete_session(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Json<serde_json::Value> {
    {
        let db = state.db.lock().unwrap();
        let _ = db.delete_session(&session_id);
    }
    let _ = state.tx.send(WsMessage::SessionDeleted(session_id));
    Json(serde_json::json!({"status": "ok"}))
}
