use anyhow::Result;
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::os::unix::process::CommandExt;
use std::process::Command;

use super::tmux;
use crate::config::load_config;
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
    let config = load_config();

    if config.tmux_mode {
        run_with_tmux(claude_args)
    } else {
        run_without_tmux(claude_args)
    }
}

fn run_without_tmux(claude_args: Vec<String>) -> Result<()> {
    ensure_daemon_running();

    let project_path = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let signal_project_path = project_path.clone();
    if let Ok(mut signals) = Signals::new([SIGHUP, SIGINT, SIGTERM]) {
        std::thread::spawn(move || {
            if signals.forever().next().is_some() {
                notify_stopped(&signal_project_path);
                std::process::exit(1);
            }
        });
    }

    let mut cmd = Command::new("claude");
    cmd.env("PLATE_SPINNER", "1");
    cmd.args(&claude_args);

    let status = cmd.status()?;

    notify_stopped(&project_path);

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

fn run_with_tmux(claude_args: Vec<String>) -> Result<()> {
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
