use anyhow::Result;

use super::{check_daemon_health, post_event, read_stdin_json};

pub async fn prompt_submit() -> Result<()> {
    let data = read_stdin_json()?;

    let client = reqwest::Client::new();
    if !check_daemon_health(&client).await {
        return Ok(());
    }

    let payload = serde_json::json!({
        "session_id": data["session_id"],
        "project_path": data["cwd"],
        "event_type": "prompt_submit",
    });

    post_event(&client, payload).await;
    Ok(())
}
