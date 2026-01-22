pub mod cli;
pub mod config;
pub mod daemon;
pub mod db;
pub mod hook;
pub mod models;
pub mod tui;

use std::process::Command;

pub fn ensure_daemon_running() {
    let client = reqwest::blocking::Client::new();
    let healthy = client
        .get(format!("{}/health", hook::DAEMON_URL))
        .timeout(std::time::Duration::from_secs(1))
        .send()
        .is_ok();

    if healthy {
        return;
    }

    let exe = std::env::current_exe().unwrap_or_else(|_| "sp".into());
    Command::new(&exe)
        .arg("daemon")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok();

    std::thread::sleep(std::time::Duration::from_secs(1));
}
