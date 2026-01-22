use anyhow::{bail, Result};
use std::process::Command;

const MIN_TMUX_VERSION: (u32, u32) = (3, 2);
const DEFAULT_SESSION: &str = "plate-spinner";

pub fn check_tmux_available() -> Result<()> {
    let output = Command::new("which").arg("tmux").output()?;
    if !output.status.success() {
        bail!(
            "tmux is required for sp run. Install it with:\n  \
             brew install tmux    # macOS\n  \
             apt install tmux     # Debian/Ubuntu"
        );
    }
    Ok(())
}

pub fn check_tmux_version() -> Result<()> {
    let output = Command::new("tmux").arg("-V").output()?;
    if !output.status.success() {
        bail!("Failed to get tmux version");
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    let version = parse_tmux_version(&version_str)?;

    if version < MIN_TMUX_VERSION {
        bail!(
            "tmux {}.{} or higher is required (found {}.{})",
            MIN_TMUX_VERSION.0,
            MIN_TMUX_VERSION.1,
            version.0,
            version.1
        );
    }
    Ok(())
}

fn parse_tmux_version(s: &str) -> Result<(u32, u32)> {
    let s = s.trim().strip_prefix("tmux ").unwrap_or(s);
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() < 2 {
        bail!("Could not parse tmux version: {}", s);
    }
    let major: u32 = parts[0]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()?;
    let minor: u32 = parts[1]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()?;
    Ok((major, minor))
}

pub fn is_inside_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

pub fn get_current_session() -> Option<String> {
    if !is_inside_tmux() {
        return None;
    }
    let output = Command::new("tmux")
        .args(["display-message", "-p", "#{session_name}"])
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

pub fn get_session_name() -> String {
    get_current_session().unwrap_or_else(|| DEFAULT_SESSION.to_string())
}

pub fn ensure_session_exists(session: &str) -> Result<()> {
    let has_session = Command::new("tmux")
        .args(["has-session", "-t", session])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !has_session {
        let status = Command::new("tmux")
            .args(["new-session", "-d", "-s", session])
            .status()?;
        if !status.success() {
            bail!("Failed to create tmux session: {}", session);
        }
    }
    Ok(())
}

pub fn generate_window_name() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64,
    );
    format!("sp-{:07x}", hasher.finish() & 0xFFFFFFF)
}

pub fn format_tmux_target(session: &str, window: &str) -> String {
    format!("{}:{}", session, window)
}
