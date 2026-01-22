use axum::{
    extract::{State, WebSocketUpgrade},
    response::Response,
};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;

use super::state::{AppState, WsMessage};

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let json = match msg {
                WsMessage::SessionUpdate(id) => {
                    serde_json::json!({"type": "session_update", "session_id": id})
                }
                WsMessage::SessionDeleted(id) => {
                    serde_json::json!({"type": "session_deleted", "session_id": id})
                }
            };
            if sender.send(Message::Text(json.to_string().into())).await.is_err() {
                break;
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(_)) = receiver.next().await {}
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}
