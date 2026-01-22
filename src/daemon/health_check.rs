use std::sync::Arc;
use std::time::Duration;

use crate::models::PlateStatus;
use crate::recovery::{is_stale, HEALTH_CHECK_INTERVAL_SECS};
use crate::state_machine::Event;

use super::state::{AppState, WsMessage};

pub fn spawn_health_checker(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(HEALTH_CHECK_INTERVAL_SECS)).await;
            check_stale_statuses(&state);
        }
    });
}

fn check_stale_statuses(state: &Arc<AppState>) {
    let stale_plates: Vec<(String, PlateStatus)> = {
        let db = state.db.lock().unwrap();
        db.get_plates()
            .unwrap_or_default()
            .into_iter()
            .filter(|p| p.status.needs_attention() && p.status != PlateStatus::Idle)
            .filter_map(|p| {
                let transcript_path = p.transcript_path.as_ref()?;
                let transcript_mtime = std::fs::metadata(transcript_path).ok()?.modified().ok()?;

                let updated_at = chrono::DateTime::parse_from_rfc3339(&p.updated_at)
                    .ok()?
                    .timestamp();
                let mtime_secs = transcript_mtime
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()?
                    .as_secs() as i64;

                if is_stale(mtime_secs, updated_at) {
                    Some((p.session_id, p.status))
                } else {
                    None
                }
            })
            .collect()
    };

    for (session_id, old_status) in stale_plates {
        let new_status = old_status.transition(&Event::HealthCheckRecovery);
        let now = chrono::Utc::now().to_rfc3339();
        {
            let db = state.db.lock().unwrap();
            let _ = db.conn().execute(
                "UPDATE plates SET status = ?, updated_at = ? WHERE session_id = ?",
                rusqlite::params![new_status.as_str(), now, session_id],
            );
        }
        let _ = state.tx.send(WsMessage::PlateUpdate(session_id));
    }
}
