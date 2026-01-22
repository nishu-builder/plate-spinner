use anyhow::Result;
use std::os::unix::process::CommandExt;
use std::process::Command;

use crate::ensure_daemon_running;
use crate::hook::DAEMON_URL;

static mut CHILD_PID: libc::pid_t = 0;
static mut PROJECT_PATH: Option<String> = None;

extern "C" fn signal_handler(sig: libc::c_int) {
    unsafe {
        if CHILD_PID > 0 {
            libc::kill(CHILD_PID, sig);
        }
        if let Some(ref path) = PROJECT_PATH {
            notify_stopped(path);
        }
        std::process::exit(0);
    }
}

fn notify_stopped(project_path: &str) {
    let _ = reqwest::blocking::Client::new()
        .post(format!("{}/plates/stopped", DAEMON_URL))
        .json(&serde_json::json!({"project_path": project_path}))
        .timeout(std::time::Duration::from_secs(2))
        .send();
}

pub fn run(claude_args: Vec<String>) -> Result<()> {
    ensure_daemon_running();

    let project_path = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());

    unsafe {
        PROJECT_PATH = Some(project_path.clone());
    }

    let pid = unsafe { libc::fork() };

    if pid == 0 {
        let mut cmd = Command::new("claude");
        cmd.args(&claude_args);
        cmd.env("PLATE_SPINNER", "1");
        let err = cmd.exec();
        eprintln!("exec failed: {}", err);
        std::process::exit(1);
    } else if pid > 0 {
        unsafe {
            CHILD_PID = pid;
            libc::signal(
                libc::SIGHUP,
                signal_handler as *const () as libc::sighandler_t,
            );
            libc::signal(
                libc::SIGTERM,
                signal_handler as *const () as libc::sighandler_t,
            );
            libc::signal(
                libc::SIGINT,
                signal_handler as *const () as libc::sighandler_t,
            );

            let mut status: libc::c_int = 0;
            libc::waitpid(pid, &mut status, 0);
        }
        notify_stopped(&project_path);
    } else {
        anyhow::bail!("fork failed");
    }

    Ok(())
}
