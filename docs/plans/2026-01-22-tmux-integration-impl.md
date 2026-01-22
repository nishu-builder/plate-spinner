# tmux Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wrap `sp run` and `sp` (TUI) in tmux for programmatic prompt injection and easy window switching.

**Architecture:** Both commands create windows in a shared tmux session (`plate-spinner` if not already in tmux, current session otherwise). The `tmux_target` is passed via env var and stored in DB for future use.

**Tech Stack:** Rust, tmux 3.2+, SQLite

---

## Task 1: Add tmux_target column to database

**Files:**
- Modify: `src/db.rs:58-78` (migrate function)

**Step 1: Add the migration**

In `src/db.rs`, add `tmux_target` check to the `migrate()` function:

```rust
fn migrate(&self) -> Result<()> {
    let columns: Vec<String> = self
        .conn
        .prepare("PRAGMA table_info(plates)")?
        .query_map([], |row| row.get(1))?
        .filter_map(|r| r.ok())
        .collect();

    if !columns.contains(&"summary".to_string()) {
        self.conn
            .execute("ALTER TABLE plates ADD COLUMN summary TEXT", [])?;
    }
    if !columns.contains(&"transcript_path".to_string()) {
        self.conn
            .execute("ALTER TABLE plates ADD COLUMN transcript_path TEXT", [])?;
    }
    if !columns.contains(&"git_branch".to_string()) {
        self.conn
            .execute("ALTER TABLE plates ADD COLUMN git_branch TEXT", [])?;
    }
    if !columns.contains(&"tmux_target".to_string()) {
        self.conn
            .execute("ALTER TABLE plates ADD COLUMN tmux_target TEXT", [])?;
    }
    Ok(())
}
```

**Step 2: Build and verify**

Run: `cargo build`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add src/db.rs
git commit -m "add tmux_target column migration"
```

---

## Task 2: Update upsert_plate to accept tmux_target

**Files:**
- Modify: `src/db.rs:85-122` (upsert_plate function)

**Step 1: Add tmux_target parameter**

Update the `upsert_plate` function signature and implementation:

```rust
#[allow(clippy::too_many_arguments)]
pub fn upsert_plate(
    &self,
    session_id: &str,
    project_path: &str,
    transcript_path: Option<&str>,
    git_branch: Option<&str>,
    tmux_target: Option<&str>,
    status: &str,
    event_type: &str,
    tool_name: Option<&str>,
    now: &str,
) -> Result<bool> {
    let existing: Option<String> = self
        .conn
        .query_row(
            "SELECT session_id FROM plates WHERE session_id = ?",
            [session_id],
            |row| row.get(0),
        )
        .ok();

    if existing.is_none() {
        let placeholder_id = format!("pending:{}", project_path);
        self.conn
            .execute("DELETE FROM plates WHERE session_id = ?", [&placeholder_id])?;
        self.conn.execute(
            "INSERT INTO plates (session_id, project_path, transcript_path, git_branch, tmux_target, status, last_event_type, last_tool, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![session_id, project_path, transcript_path, git_branch, tmux_target, status, event_type, tool_name, now, now],
        )?;
        Ok(false)
    } else {
        self.conn.execute(
            "UPDATE plates SET status = ?, last_event_type = ?, last_tool = COALESCE(?, last_tool), transcript_path = COALESCE(?, transcript_path), git_branch = COALESCE(?, git_branch), tmux_target = COALESCE(?, tmux_target), updated_at = ? WHERE session_id = ?",
            params![status, event_type, tool_name, transcript_path, git_branch, tmux_target, now, session_id],
        )?;
        Ok(true)
    }
}
```

**Step 2: Build (will fail - callers need updating)**

Run: `cargo build`
Expected: Compile errors in `handlers.rs` (missing argument)

**Step 3: Commit partial progress**

```bash
git add src/db.rs
git commit -m "add tmux_target param to upsert_plate"
```

---

## Task 3: Update HookEvent model

**Files:**
- Modify: `src/models.rs:86-101` (HookEvent struct)

**Step 1: Add tmux_target field**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub session_id: String,
    pub project_path: String,
    pub event_type: String,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_params: Option<serde_json::Value>,
    #[serde(default)]
    pub transcript_path: Option<String>,
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub tmux_target: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}
```

**Step 2: Build (still fails - handlers.rs)**

Run: `cargo build`
Expected: Still fails on handlers.rs

**Step 3: Commit**

```bash
git add src/models.rs
git commit -m "add tmux_target to HookEvent model"
```

---

## Task 4: Update daemon handler to pass tmux_target

**Files:**
- Modify: `src/daemon/handlers.rs:99-141` (post_event function)

**Step 1: Pass tmux_target to upsert_plate**

```rust
pub async fn post_event(
    State(state): State<Arc<AppState>>,
    Json(event): Json<HookEvent>,
) -> Json<serde_json::Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let status = determine_status(&event);

    {
        let db = state.db.lock().unwrap();
        let _ = db.upsert_plate(
            &event.session_id,
            &event.project_path,
            event.transcript_path.as_deref(),
            event.git_branch.as_deref(),
            event.tmux_target.as_deref(),
            status.as_str(),
            &event.event_type,
            event.tool_name.as_deref(),
            &now,
        );

        if event.tool_name.as_deref() == Some("TodoWrite") {
            if let Some(params) = &event.tool_params {
                if let Some(todos) = params.get("todos") {
                    let _ = db.upsert_todos(&event.session_id, &todos.to_string(), &now);
                }
            }
        }

        let _ = db.insert_event(
            &event.session_id,
            &event.event_type,
            &serde_json::to_string(&event).unwrap_or_default(),
            &now,
        );
    }

    maybe_summarize(state.clone(), event.clone(), status);

    let _ = state
        .tx
        .send(WsMessage::PlateUpdate(event.session_id.clone()));
    Json(serde_json::json!({"status": "ok"}))
}
```

**Step 2: Build and verify**

Run: `cargo build`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/daemon/handlers.rs
git commit -m "pass tmux_target through handler to db"
```

---

## Task 5: Update session_start hook to read env var

**Files:**
- Modify: `src/hook/session_start.rs:6-27`

**Step 1: Read PLATE_SPINNER_TMUX_TARGET from environment**

```rust
pub async fn session_start() -> Result<()> {
    let data = read_stdin_json()?;

    let client = reqwest::Client::new();
    if !check_daemon_health(&client).await {
        return Ok(());
    }

    let cwd = data["cwd"].as_str().unwrap_or(".");
    let git_branch = get_git_branch(cwd);
    let tmux_target = std::env::var("PLATE_SPINNER_TMUX_TARGET").ok();

    let payload = serde_json::json!({
        "session_id": data["session_id"],
        "project_path": data["cwd"],
        "event_type": "session_start",
        "transcript_path": data["transcript_path"],
        "git_branch": git_branch,
        "tmux_target": tmux_target,
    });

    post_event(&client, payload).await;
    Ok(())
}
```

**Step 2: Build and verify**

Run: `cargo build`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/hook/session_start.rs
git commit -m "read PLATE_SPINNER_TMUX_TARGET in session_start hook"
```

---

## Task 6: Create tmux helper module

**Files:**
- Create: `src/cli/tmux.rs`
- Modify: `src/cli/mod.rs` (add module)

**Step 1: Create the tmux helper module**

Create `src/cli/tmux.rs`:

```rust
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
    hasher.write_u64(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64);
    format!("sp-{:07x}", hasher.finish() & 0xFFFFFFF)
}

pub fn format_tmux_target(session: &str, window: &str) -> String {
    format!("{}:{}", session, window)
}
```

**Step 2: Add module to cli/mod.rs**

Check what's in `src/cli/mod.rs` first, then add `pub mod tmux;`

**Step 3: Build and verify**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/cli/tmux.rs src/cli/mod.rs
git commit -m "add tmux helper module"
```

---

## Task 7: Rewrite sp run to use tmux

**Files:**
- Modify: `src/cli/run.rs`

**Step 1: Replace the entire run.rs with tmux-based implementation**

```rust
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
        "-e", "PLATE_SPINNER=1",
        "-e", &format!("PLATE_SPINNER_TMUX_TARGET={}", tmux_target),
        "--",
        "sh", "-c", &format!("claude{}; exit", claude_args_str),
    ]);

    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("Failed to create tmux window");
    }

    if !in_tmux {
        let err = Command::new("tmux")
            .args(["attach", "-t", &tmux_target])
            .exec();
        eprintln!("Failed to attach to tmux: {}", err);
    }

    notify_stopped(&project_path);
    Ok(())
}
```

**Step 2: Add shell_words dependency**

Run: `cargo add shell_words`

**Step 3: Build and verify**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Manual test**

Run: `cargo run -- run`
Expected: Creates tmux window, runs claude, auto-closes on exit

**Step 5: Commit**

```bash
git add src/cli/run.rs Cargo.toml Cargo.lock
git commit -m "rewrite sp run to use tmux"
```

---

## Task 8: Wrap TUI in tmux

**Files:**
- Modify: `src/main.rs:163-187` (None match arm)

**Step 1: Update the TUI launch to use tmux**

Replace the `None =>` match arm:

```rust
None => {
    use plate_spinner::cli::tmux;

    if let Err(e) = tmux::check_tmux_available() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
    if let Err(e) = tmux::check_tmux_version() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    let in_tmux = tmux::is_inside_tmux();
    let session = tmux::get_session_name();

    if !in_tmux {
        if let Err(e) = tmux::ensure_session_exists(&session) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    let exe = std::env::current_exe().unwrap_or_else(|_| "sp".into());
    let exe_str = exe.to_string_lossy();

    let mut cmd = Command::new("tmux");
    cmd.args(["new-window", "-n", "dashboard"]);

    if !in_tmux {
        cmd.args(["-t", &format!("{}:", &session)]);
    }

    cmd.args(["--", &*exe_str, "tui"]);

    let status = cmd.status().expect("Failed to run tmux");
    if !status.success() {
        eprintln!("Failed to create tmux window");
        std::process::exit(1);
    }

    if !in_tmux {
        let err = Command::new("tmux")
            .args(["attach", "-t", &format!("{}:dashboard", session)])
            .exec();
        eprintln!("Failed to attach to tmux: {}", err);
        std::process::exit(1);
    }
}
```

**Step 2: Add Tui subcommand**

Add a new `Tui` variant to the `Commands` enum and handle it:

In the enum:
```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing variants ...
    #[command(about = "Run TUI directly (internal)", hide = true)]
    Tui,
}
```

In the match:
```rust
Some(Commands::Tui) => {
    ensure_daemon_running();
    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    let resume = rt.block_on(async { plate_spinner::tui::run().await });

    match resume {
        Ok(Some((session_id, project_path))) => {
            if let Err(e) = std::env::set_current_dir(&project_path) {
                eprintln!("Failed to change directory: {}", e);
                std::process::exit(1);
            }
            let err = Command::new("claude")
                .arg("--resume")
                .arg(&session_id)
                .exec();
            eprintln!("Failed to exec claude: {}", err);
            std::process::exit(1);
        }
        Ok(None) => {}
        Err(e) => {
            eprintln!("TUI error: {}", e);
            std::process::exit(1);
        }
    }
}
```

**Step 3: Build and verify**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Manual test**

Run: `cargo run`
Expected: Creates tmux window with dashboard, can switch with ctrl-b

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "wrap TUI in tmux"
```

---

## Task 9: Final integration test

**Step 1: Full workflow test**

1. Start fresh (kill any existing plate-spinner tmux session): `tmux kill-session -t plate-spinner 2>/dev/null`
2. Run `sp` - should create plate-spinner session with dashboard window
3. Press `ctrl-b c` to create new window, run `sp run` - should create sp-xxx window
4. Press `ctrl-b 0` - should go back to dashboard
5. Press `ctrl-b 1` - should go to Claude session
6. Exit Claude - window should close
7. Exit dashboard - window should close

**Step 2: Commit final state if needed**

```bash
git add -A
git commit -m "tmux integration complete"
```
