pub mod session_start;
pub mod stop;
pub mod tool_use;

pub use session_start::session_start;
pub use stop::stop;
pub use tool_use::{post_tool_use, pre_tool_use};

use anyhow::Result;
use std::io::Read;

pub const DAEMON_URL: &str = "http://localhost:7890";

pub fn read_stdin_json() -> Result<serde_json::Value> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    Ok(serde_json::from_str(&input)?)
}

pub async fn check_daemon_health(client: &reqwest::Client) -> bool {
    client
        .get(format!("{}/health", DAEMON_URL))
        .timeout(std::time::Duration::from_secs(1))
        .send()
        .await
        .is_ok()
}

pub async fn post_event(client: &reqwest::Client, payload: serde_json::Value) {
    let _ = client
        .post(format!("{}/events", DAEMON_URL))
        .json(&payload)
        .send()
        .await;
}
