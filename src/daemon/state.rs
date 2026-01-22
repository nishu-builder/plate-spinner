use crate::db::Database;
use std::sync::Mutex;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum WsMessage {
    PlateUpdate(String),
    PlateDeleted(String),
}

pub struct AppState {
    pub db: Mutex<Database>,
    pub tx: broadcast::Sender<WsMessage>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            db: Mutex::new(db),
            tx,
        }
    }
}
