pub mod cli;
pub mod config;
pub mod daemon;
pub mod db;
pub mod hook;
pub mod models;
pub mod recovery;
pub mod state_machine;
pub mod tui;

use std::process::Command;

pub fn build_version() -> String {
    let version = env!("CARGO_PKG_VERSION");
    if cfg!(debug_assertions) {
        format!("{}-dev+{}", version, env!("BUILD_TIMESTAMP"))
    } else {
        version.to_string()
    }
}

fn kill_daemon() {
    let client = reqwest::blocking::Client::new();
    let _ = client
        .post(format!("{}/shutdown", hook::DAEMON_URL))
        .timeout(std::time::Duration::from_secs(1))
        .send();
    std::thread::sleep(std::time::Duration::from_millis(500));
}

fn spawn_daemon() {
    let exe = std::env::current_exe().unwrap_or_else(|_| "sp".into());
    Command::new(&exe)
        .arg("daemon")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok();
    std::thread::sleep(std::time::Duration::from_secs(1));
}

pub fn ensure_daemon_running() {
    let client = reqwest::blocking::Client::new();
    let my_version = build_version();

    let response = client
        .get(format!("{}/health", hook::DAEMON_URL))
        .timeout(std::time::Duration::from_secs(1))
        .send()
        .ok()
        .and_then(|r| r.json::<serde_json::Value>().ok());

    match response {
        Some(json) => {
            let daemon_version = json.get("version").and_then(|v| v.as_str());
            if daemon_version != Some(&my_version) {
                kill_daemon();
                spawn_daemon();
            }
        }
        None => {
            spawn_daemon();
        }
    }
}
