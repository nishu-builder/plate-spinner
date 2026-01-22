use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::state::{AppState, WsMessage};
use super::summarizer;
use crate::models::{HookEvent, PlateStatus};
use crate::state_machine::Event;

#[derive(Serialize)]
pub struct StatusResponse {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key_configured: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hooks_installed: Option<bool>,
}

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": crate::build_version()
    }))
}

pub async fn shutdown() -> Json<serde_json::Value> {
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        std::process::exit(0);
    });
    Json(serde_json::json!({"status": "ok"}))
}

pub async fn status() -> Json<StatusResponse> {
    let api_key_configured = summarizer::get_api_key().is_some();
    Json(StatusResponse {
        status: "ok".to_string(),
        api_key_configured: Some(api_key_configured),
        hooks_installed: Some(true),
    })
}

fn determine_status(event: &HookEvent) -> PlateStatus {
    let sm_event = Event::from_hook(
        &event.event_type,
        event.tool_name.as_deref(),
        event.error.as_deref(),
    );
    PlateStatus::Running.transition(&sm_event)
}

fn maybe_summarize(state: Arc<AppState>, event: HookEvent, status: PlateStatus) {
    let should_summarize = {
        let db = state.db.lock().unwrap();

        let needs_attention = matches!(
            status,
            PlateStatus::AwaitingInput | PlateStatus::AwaitingApproval | PlateStatus::Idle
        );

        if needs_attention {
            true
        } else if event.event_type == "tool_call" {
            let event_count = db.get_event_count(&event.session_id).unwrap_or(0);
            event_count > 0 && event_count % 5 == 0
        } else {
            db.get_summary(&event.session_id).ok().flatten().is_none()
        }
    };

    if !should_summarize {
        return;
    }

    let transcript_path = event.transcript_path.clone().or_else(|| {
        let db = state.db.lock().unwrap();
        db.get_transcript_path(&event.session_id).ok().flatten()
    });

    let Some(transcript) = transcript_path else {
        return;
    };

    let session_id = event.session_id.clone();
    let tx = state.tx.clone();

    let cached_goal = {
        let db = state.db.lock().unwrap();
        db.get_goal(&session_id).ok().flatten()
    };

    tokio::task::spawn_blocking(move || {
        if let Some(result) = summarizer::summarize_session(&transcript, cached_goal.as_deref()) {
            let db = state.db.lock().unwrap();
            if let Some(goal) = result.goal {
                let _ = db.set_goal(&session_id, &goal);
            }
            if db.set_summary(&session_id, &result.summary).is_ok() {
                let _ = tx.send(WsMessage::PlateUpdate(session_id));
            }
        }
    });
}

pub async fn post_event(
    State(state): State<Arc<AppState>>,
    Json(event): Json<HookEvent>,
) -> Json<serde_json::Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let status = determine_status(&event);

    {
        let db = state.db.lock().unwrap();
        let _ = db.upsert_plate(
            &event.session_id,
            &event.project_path,
            event.transcript_path.as_deref(),
            event.git_branch.as_deref(),
            event.tmux_target.as_deref(),
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

    maybe_summarize(state.clone(), event.clone(), status);

    let _ = state
        .tx
        .send(WsMessage::PlateUpdate(event.session_id.clone()));
    Json(serde_json::json!({"status": "ok"}))
}

pub async fn get_plates(State(state): State<Arc<AppState>>) -> Json<Vec<crate::models::Plate>> {
    let db = state.db.lock().unwrap();
    Json(db.get_plates().unwrap_or_default())
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    project_path: String,
}

pub async fn register_plate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Json<serde_json::Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let placeholder_id = {
        let db = state.db.lock().unwrap();
        db.register_placeholder(&req.project_path, &now)
            .unwrap_or_default()
    };
    let _ = state
        .tx
        .send(WsMessage::PlateUpdate(placeholder_id.clone()));
    Json(serde_json::json!({"status": "ok", "placeholder_id": placeholder_id}))
}

pub async fn mark_stopped(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Json<serde_json::Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let plate_ids = {
        let db = state.db.lock().unwrap();
        db.mark_stopped(&req.project_path, &now).unwrap_or_default()
    };
    for plate_id in &plate_ids {
        let _ = state.tx.send(WsMessage::PlateUpdate(plate_id.clone()));
    }
    Json(serde_json::json!({"status": "ok", "count": plate_ids.len()}))
}

pub async fn delete_plate(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Json<serde_json::Value> {
    {
        let db = state.db.lock().unwrap();
        let _ = db.delete_plate(&session_id);
    }
    let _ = state.tx.send(WsMessage::PlateDeleted(session_id));
    Json(serde_json::json!({"status": "ok"}))
}
