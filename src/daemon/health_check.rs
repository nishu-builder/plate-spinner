use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::models::PlateStatus;
use crate::recovery::{is_running_stale, is_stale, HEALTH_CHECK_INTERVAL_SECS};
use crate::state_machine::Event;

use super::state::{AppState, WsMessage};

const SLEEP_DETECTION_MULTIPLIER: u64 = 3;
const POST_WAKE_GRACE_PERIOD_SECS: i64 = 10;

static LAST_HEALTH_CHECK_TIME: AtomicI64 = AtomicI64::new(0);
static WAKE_GRACE_UNTIL: AtomicI64 = AtomicI64::new(0);

pub fn spawn_health_checker(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(HEALTH_CHECK_INTERVAL_SECS)).await;
            check_stale_statuses(&state);
        }
    });
}

fn check_stale_statuses(state: &Arc<AppState>) {
    let now_secs = chrono::Utc::now().timestamp();
    let last_check = LAST_HEALTH_CHECK_TIME.swap(now_secs, Ordering::Relaxed);

    let expected_gap = HEALTH_CHECK_INTERVAL_SECS as i64;
    let sleep_threshold = expected_gap * SLEEP_DETECTION_MULTIPLIER as i64;
    if last_check > 0 && (now_secs - last_check) > sleep_threshold {
        WAKE_GRACE_UNTIL.store(now_secs + POST_WAKE_GRACE_PERIOD_SECS, Ordering::Relaxed);
    }

    let in_grace_period = now_secs < WAKE_GRACE_UNTIL.load(Ordering::Relaxed);

    let stale_plates: Vec<(String, PlateStatus)> = {
        let db = state.db.lock().unwrap();
        db.get_plates()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|p| {
                let transcript_path = p.transcript_path.as_ref()?;
                let transcript_mtime = std::fs::metadata(transcript_path).ok()?.modified().ok()?;
                let mtime_secs = transcript_mtime
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()?
                    .as_secs() as i64;

                if p.status == PlateStatus::Running {
                    if in_grace_period {
                        return None;
                    }
                    if is_running_stale(mtime_secs, now_secs) {
                        return Some((p.session_id, p.status));
                    }
                } else if p.status.needs_attention() && p.status != PlateStatus::Idle {
                    let updated_at = chrono::DateTime::parse_from_rfc3339(&p.updated_at)
                        .ok()?
                        .timestamp();
                    if is_stale(mtime_secs, updated_at) {
                        return Some((p.session_id, p.status));
                    }
                }
                None
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
