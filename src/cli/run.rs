use anyhow::Result;
use std::os::unix::process::CommandExt;
use std::process::Command;

use super::tmux;
use crate::ensure_daemon_running;
use crate::hook::DAEMON_URL;

fn notify_stopped(project_path: &str) {
    let _ = reqwest::blocking::Client::new()
        .post(format!("{}/plates/stopped", DAEMON_URL))
        .json(&serde_json::json!({"project_path": project_path}))
        .timeout(std::time::Duration::from_secs(2))
        .send();
}

pub fn run(claude_args: Vec<String>) -> Result<()> {
    tmux::check_tmux_available()?;
    tmux::check_tmux_version()?;
    ensure_daemon_running();

    let project_path = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let session = tmux::get_session_name();
    let window = tmux::generate_window_name();
    let tmux_target = tmux::format_tmux_target(&session, &window);
    let in_tmux = tmux::is_inside_tmux();

    if !in_tmux {
        tmux::ensure_session_exists(&session)?;
    }

    let claude_args_str = if claude_args.is_empty() {
        String::new()
    } else {
        format!(" {}", shell_words::join(&claude_args))
    };

    let mut cmd = Command::new("tmux");
    cmd.args(["new-window", "-n", &window]);

    if !in_tmux {
        cmd.args(["-t", &format!("{}:", &session)]);
    }

    cmd.args([
        "-e",
        "PLATE_SPINNER=1",
        "-e",
        &format!("PLATE_SPINNER_TMUX_TARGET={}", tmux_target),
        "--",
        "sh",
        "-c",
        &format!("claude{}; exit", claude_args_str),
    ]);

    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("Failed to create tmux window");
    }

    if !in_tmux {
        // Use grouped session so this terminal has independent window view
        let grouped = tmux::generate_grouped_session_name();
        let err = Command::new("tmux")
            .args([
                "new-session",
                "-t",
                &session,
                "-s",
                &grouped,
                ";",
                "select-window",
                "-t",
                &window,
            ])
            .exec();
        eprintln!("Failed to attach to tmux: {}", err);
    }

    notify_stopped(&project_path);
    Ok(())
}
