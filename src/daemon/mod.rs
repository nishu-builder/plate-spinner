pub mod handlers;
pub mod state;
pub mod websocket;

use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use state::AppState;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/status", get(handlers::status))
        .route("/events", post(handlers::post_event))
        .route("/plates", get(handlers::get_plates))
        .route("/plates/register", post(handlers::register_plate))
        .route("/plates/stopped", post(handlers::mark_stopped))
        .route("/plates/{session_id}", delete(handlers::delete_plate))
        .route("/ws", get(websocket::websocket_handler))
        .with_state(state)
}

pub async fn run(state: Arc<AppState>, port: u16) -> anyhow::Result<()> {
    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
