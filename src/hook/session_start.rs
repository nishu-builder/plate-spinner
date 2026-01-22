use anyhow::Result;
use std::process::Command;

use super::{check_daemon_health, post_event, read_stdin_json};

pub async fn session_start() -> Result<()> {
    let data = read_stdin_json()?;

    let client = reqwest::Client::new();
    if !check_daemon_health(&client).await {
        return Ok(());
    }

    let cwd = data["cwd"].as_str().unwrap_or(".");
    let git_branch = get_git_branch(cwd);

    let payload = serde_json::json!({
        "session_id": data["session_id"],
        "project_path": data["cwd"],
        "event_type": "session_start",
        "transcript_path": data["transcript_path"],
        "git_branch": git_branch,
    });

    post_event(&client, payload).await;
    Ok(())
}

fn get_git_branch(cwd: &str) -> Option<String> {
    Command::new("git")
        .args(["-C", cwd, "rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}
