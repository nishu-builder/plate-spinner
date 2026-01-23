pub const HEALTH_CHECK_INTERVAL_SECS: u64 = 10;
pub const STALENESS_THRESHOLD_SECS: i64 = 2;
pub const RUNNING_STALENESS_THRESHOLD_SECS: i64 = 30;
pub const MAX_RECOVERY_TIME_SECS: u64 =
    HEALTH_CHECK_INTERVAL_SECS + STALENESS_THRESHOLD_SECS as u64;

pub fn is_stale(transcript_mtime_secs: i64, last_event_time_secs: i64) -> bool {
    transcript_mtime_secs > last_event_time_secs + STALENESS_THRESHOLD_SECS
}

pub fn is_running_stale(transcript_mtime_secs: i64, now_secs: i64) -> bool {
    now_secs - transcript_mtime_secs > RUNNING_STALENESS_THRESHOLD_SECS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_recovery_time_is_12_seconds() {
        assert_eq!(MAX_RECOVERY_TIME_SECS, 12);
    }

    #[test]
    fn not_stale_when_equal() {
        assert!(!is_stale(100, 100));
    }

    #[test]
    fn not_stale_within_threshold() {
        assert!(!is_stale(102, 100));
    }

    #[test]
    fn stale_when_past_threshold() {
        assert!(is_stale(103, 100));
    }

    #[test]
    fn stale_when_well_past_threshold() {
        assert!(is_stale(200, 100));
    }

    #[test]
    fn not_stale_when_transcript_older() {
        assert!(!is_stale(90, 100));
    }

    #[test]
    fn running_not_stale_within_threshold() {
        assert!(!is_running_stale(100, 130));
    }

    #[test]
    fn running_stale_when_past_threshold() {
        assert!(is_running_stale(100, 131));
    }

    #[test]
    fn running_stale_when_well_past_threshold() {
        assert!(is_running_stale(100, 200));
    }
}
