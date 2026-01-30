use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::models::PlateStatus;
use crate::recovery::{
    is_running_stale, is_stale, HEALTH_CHECK_INTERVAL_SECS, RUNNING_ABSOLUTE_TIMEOUT_SECS,
};
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

fn transcript_shows_completion(transcript_path: &str) -> bool {
    let file = match std::fs::File::open(transcript_path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let file_size = match file.metadata() {
        Ok(m) => m.len(),
        Err(_) => return false,
    };

    let mut reader = BufReader::new(file);

    if file_size > 65536 {
        let _ = reader.seek(SeekFrom::End(-65536));
        let mut partial = String::new();
        let _ = reader.read_line(&mut partial);
    }

    let mut last_line = String::new();
    let mut buf = String::new();
    while reader.read_line(&mut buf).unwrap_or(0) > 0 {
        let trimmed = buf.trim();
        if !trimmed.is_empty() {
            last_line.clear();
            last_line.push_str(trimmed);
        }
        buf.clear();
    }

    if last_line.is_empty() {
        return false;
    }

    let entry: serde_json::Value = match serde_json::from_str(&last_line) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let entry_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match entry_type {
        "summary" => true,
        "assistant" => {
            let stop_reason = entry
                .get("message")
                .and_then(|m| m.get("stop_reason"))
                .unwrap_or(&serde_json::Value::Null);
            stop_reason == "end_turn"
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    struct TempTranscript(std::path::PathBuf);

    impl TempTranscript {
        fn new(content: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "sp-test-{}-{}.jsonl",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(content.as_bytes()).unwrap();
            Self(path)
        }

        fn path(&self) -> &str {
            self.0.to_str().unwrap()
        }
    }

    impl Drop for TempTranscript {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn completion_on_end_turn() {
        let f = TempTranscript::new(
            r#"{"type":"assistant","message":{"stop_reason":"end_turn","content":[]}}"#,
        );
        assert!(transcript_shows_completion(f.path()));
    }

    #[test]
    fn completion_on_summary() {
        let f = TempTranscript::new(r#"{"type":"summary","summary":"did stuff"}"#);
        assert!(transcript_shows_completion(f.path()));
    }

    #[test]
    fn no_completion_on_tool_use() {
        let f = TempTranscript::new(
            r#"{"type":"assistant","message":{"stop_reason":"tool_use","content":[]}}"#,
        );
        assert!(!transcript_shows_completion(f.path()));
    }

    #[test]
    fn no_completion_on_null_stop_reason() {
        let f = TempTranscript::new(
            r#"{"type":"assistant","message":{"stop_reason":null,"content":[]}}"#,
        );
        assert!(!transcript_shows_completion(f.path()));
    }

    #[test]
    fn no_completion_on_progress() {
        let f = TempTranscript::new(
            r#"{"type":"progress","data":{"type":"bash_progress","elapsedTimeSeconds":5}}"#,
        );
        assert!(!transcript_shows_completion(f.path()));
    }

    #[test]
    fn no_completion_on_user_entry() {
        let f =
            TempTranscript::new(r#"{"type":"user","message":{"role":"user","content":"hello"}}"#);
        assert!(!transcript_shows_completion(f.path()));
    }

    #[test]
    fn reads_last_line_of_multiline_transcript() {
        let f = TempTranscript::new(
            &[
                r#"{"type":"user","message":{"role":"user","content":"hello"}}"#,
                r#"{"type":"assistant","message":{"stop_reason":"end_turn","content":[]}}"#,
            ]
            .join("\n"),
        );
        assert!(transcript_shows_completion(f.path()));
    }

    #[test]
    fn no_completion_on_empty_file() {
        let f = TempTranscript::new("");
        assert!(!transcript_shows_completion(f.path()));
    }

    #[test]
    fn no_completion_on_missing_file() {
        assert!(!transcript_shows_completion("/nonexistent/path.jsonl"));
    }
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
                    let updated_secs = chrono::DateTime::parse_from_rfc3339(&p.updated_at)
                        .ok()?
                        .timestamp();
                    let last_activity = mtime_secs.max(updated_secs);
                    if is_running_stale(last_activity, now_secs) {
                        let timed_out = now_secs - last_activity > RUNNING_ABSOLUTE_TIMEOUT_SECS;
                        if transcript_shows_completion(transcript_path) || timed_out {
                            return Some((p.session_id, p.status));
                        }
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
