use anyhow::Result;

use super::{check_daemon_health, post_event, read_stdin_json};

pub async fn pre_tool_use() -> Result<()> {
    tool_event("tool_start").await
}

pub async fn post_tool_use() -> Result<()> {
    tool_event("tool_call").await
}

async fn tool_event(event_type: &str) -> Result<()> {
    let data = read_stdin_json()?;

    let client = reqwest::Client::new();
    if !check_daemon_health(&client).await {
        return Ok(());
    }

    let payload = serde_json::json!({
        "session_id": data["session_id"],
        "project_path": data["cwd"],
        "event_type": event_type,
        "tool_name": data["tool_name"],
        "tool_params": data["tool_input"],
    });

    post_event(&client, payload).await;
    Ok(())
}
