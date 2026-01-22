# Plate-Spinner Rust Rewrite Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rewrite plate-spinner from Python to Rust as a single self-contained binary.

**Architecture:** CLI (`sp`) with subcommands for daemon, TUI, hooks, and session management. Daemon uses Axum for HTTP/WebSocket, rusqlite for persistence. TUI uses Ratatui. Sounds embedded via `include_bytes!`.

**Tech Stack:** Rust, Axum, Ratatui, rusqlite, reqwest, clap, tokio, rodio, serde

---

## Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Delete: `pyproject.toml`, `src/plate_spinner/`, `tests/`, `uv.lock`, `.python-version`

**Step 1: Create Cargo.toml**

```toml
[package]
name = "plate-spinner"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "sp"
path = "src/main.rs"

[dependencies]
anyhow = "1"
axum = { version = "0.7", features = ["ws"] }
clap = { version = "4", features = ["derive"] }
crossterm = "0.28"
ratatui = "0.29"
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
rodio = "0.19"
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
toml = "0.8"
chrono = { version = "0.4", features = ["serde"] }
futures-util = "0.3"
tokio-tungstenite = "0.24"

[profile.release]
lto = true
strip = true
```

**Step 2: Create src/main.rs stub**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sp", about = "Dashboard for managing Claude Code sessions")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Daemon,
    Run {
        #[arg(trailing_var_arg = true)]
        claude_args: Vec<String>,
    },
    Sessions,
    Install,
    Kill,
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommands>,
    },
    Hook {
        #[command(subcommand)]
        hook_type: HookCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    Path,
    Export,
    Import { file: String },
}

#[derive(Subcommand)]
enum HookCommands {
    SessionStart,
    PreToolUse,
    PostToolUse,
    Stop,
}

fn main() {
    let cli = Cli::parse();
    println!("plate-spinner stub");
}
```

**Step 3: Create src/lib.rs stub**

```rust
pub mod models;
pub mod config;
```

**Step 4: Remove Python files**

```bash
rm -rf src/plate_spinner tests pyproject.toml uv.lock .python-version .venv
```

**Step 5: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add -A
git commit -m "Scaffold Rust project structure"
```

---

## Task 2: Data Models

**Files:**
- Create: `src/models.rs`

**Step 1: Create models.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    #[default]
    Starting,
    Running,
    Idle,
    AwaitingInput,
    AwaitingApproval,
    Error,
    Closed,
}

impl SessionStatus {
    pub fn from_tool(tool_name: &str) -> Self {
        match tool_name {
            "AskUserQuestion" => Self::AwaitingInput,
            "ExitPlanMode" => Self::AwaitingApproval,
            _ => Self::Running,
        }
    }

    pub fn needs_attention(&self) -> bool {
        matches!(
            self,
            Self::AwaitingInput | Self::AwaitingApproval | Self::Idle | Self::Error
        )
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Idle => "idle",
            Self::AwaitingInput => "awaiting_input",
            Self::AwaitingApproval => "awaiting_approval",
            Self::Error => "error",
            Self::Closed => "closed",
        }
    }

    pub fn icon(&self) -> char {
        match self {
            Self::Starting => '.',
            Self::Running => '>',
            Self::Idle => '-',
            Self::AwaitingInput => '?',
            Self::AwaitingApproval => '!',
            Self::Error => 'X',
            Self::Closed => 'x',
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Starting => "start",
            Self::Running => "running",
            Self::Idle => "idle",
            Self::AwaitingInput => "input",
            Self::AwaitingApproval => "approve",
            Self::Error => "error",
            Self::Closed => "closed",
        }
    }
}

impl std::str::FromStr for SessionStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "starting" => Ok(Self::Starting),
            "running" => Ok(Self::Running),
            "idle" => Ok(Self::Idle),
            "awaiting_input" => Ok(Self::AwaitingInput),
            "awaiting_approval" => Ok(Self::AwaitingApproval),
            "error" => Ok(Self::Error),
            "closed" => Ok(Self::Closed),
            _ => Err(()),
        }
    }
}

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
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub project_path: String,
    #[serde(default)]
    pub transcript_path: Option<String>,
    #[serde(default)]
    pub git_branch: Option<String>,
    pub status: SessionStatus,
    #[serde(default)]
    pub last_event_type: Option<String>,
    #[serde(default)]
    pub last_tool: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub todo_progress: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Session {
    pub fn project_name(&self) -> &str {
        self.project_path
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(&self.project_path)
    }
}
```

**Step 2: Update lib.rs**

```rust
pub mod models;
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/models.rs src/lib.rs
git commit -m "Add data models"
```

---

## Task 3: Config Module

**Files:**
- Create: `src/config.rs`
- Copy: `src/plate_spinner/sounds/*.wav` -> `sounds/`

**Step 1: Copy sound files from original**

```bash
mkdir -p sounds
cp ../src/plate_spinner/sounds/*.wav sounds/ 2>/dev/null || cp ../../src/plate_spinner/sounds/*.wav sounds/
```

**Step 2: Create config.rs**

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub sounds: SoundsConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_tap")]
    pub awaiting_input: String,
    #[serde(default = "default_bell")]
    pub awaiting_approval: String,
    #[serde(default = "default_error")]
    pub error: String,
    #[serde(default = "default_pop")]
    pub idle: String,
    #[serde(default = "default_none")]
    pub closed: String,
}

impl Default for SoundsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            awaiting_input: "tap".to_string(),
            awaiting_approval: "bell".to_string(),
            error: "error".to_string(),
            idle: "pop".to_string(),
            closed: "none".to_string(),
        }
    }
}

fn default_true() -> bool { true }
fn default_tap() -> String { "tap".to_string() }
fn default_bell() -> String { "bell".to_string() }
fn default_error() -> String { "error".to_string() }
fn default_pop() -> String { "pop".to_string() }
fn default_none() -> String { "none".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_theme")]
    pub name: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "textual-dark".to_string(),
        }
    }
}

fn default_theme() -> String { "textual-dark".to_string() }

pub fn get_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".config")
        .join("plate-spinner")
        .join("config.toml")
}

pub fn get_data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".plate-spinner")
}

pub fn load_config() -> Config {
    let path = get_config_path();
    if !path.exists() {
        return Config::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(config: &Config) -> anyhow::Result<()> {
    let path = get_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, toml::to_string_pretty(config)?)?;
    Ok(())
}

pub const AVAILABLE_SOUNDS: &[&str] = &["alert", "bell", "click", "error", "long-pop", "pop", "tap", "none"];

pub const SOUND_ALERT: &[u8] = include_bytes!("../sounds/alert.wav");
pub const SOUND_BELL: &[u8] = include_bytes!("../sounds/bell.wav");
pub const SOUND_CLICK: &[u8] = include_bytes!("../sounds/click.wav");
pub const SOUND_ERROR: &[u8] = include_bytes!("../sounds/error.wav");
pub const SOUND_LONG_POP: &[u8] = include_bytes!("../sounds/long-pop.wav");
pub const SOUND_POP: &[u8] = include_bytes!("../sounds/pop.wav");
pub const SOUND_TAP: &[u8] = include_bytes!("../sounds/tap.wav");

pub fn get_sound_bytes(name: &str) -> Option<&'static [u8]> {
    match name {
        "alert" => Some(SOUND_ALERT),
        "bell" => Some(SOUND_BELL),
        "click" => Some(SOUND_CLICK),
        "error" => Some(SOUND_ERROR),
        "long-pop" => Some(SOUND_LONG_POP),
        "pop" => Some(SOUND_POP),
        "tap" => Some(SOUND_TAP),
        _ => None,
    }
}

pub fn play_sound(name: &str) {
    let Some(bytes) = get_sound_bytes(name) else {
        return;
    };
    let bytes = bytes.to_vec();
    std::thread::spawn(move || {
        if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
            if let Ok(source) = rodio::Decoder::new(std::io::Cursor::new(bytes)) {
                use rodio::Source;
                let _ = handle.play_raw(source.convert_samples());
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    });
}
```

**Step 3: Add dirs dependency to Cargo.toml**

Add to `[dependencies]`:
```toml
dirs = "5"
```

**Step 4: Update lib.rs**

```rust
pub mod config;
pub mod models;
```

**Step 5: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add Cargo.toml src/config.rs src/lib.rs sounds/
git commit -m "Add config module with embedded sounds"
```

---

## Task 4: Database Layer

**Files:**
- Create: `src/db.rs`

**Step 1: Create db.rs**

```rust
use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::Path;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
    session_id TEXT PRIMARY KEY,
    project_path TEXT NOT NULL,
    transcript_path TEXT,
    git_branch TEXT,
    status TEXT NOT NULL DEFAULT 'running',
    last_event_type TEXT,
    last_tool TEXT,
    summary TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS todos (
    session_id TEXT PRIMARY KEY REFERENCES sessions(session_id),
    todos_json TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id);
"#;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(SCHEMA)?;
        self.migrate()?;
        Ok(())
    }

    fn migrate(&self) -> Result<()> {
        let columns: Vec<String> = self.conn
            .prepare("PRAGMA table_info(sessions)")?
            .query_map([], |row| row.get(1))?
            .filter_map(|r| r.ok())
            .collect();

        if !columns.contains(&"summary".to_string()) {
            self.conn.execute("ALTER TABLE sessions ADD COLUMN summary TEXT", [])?;
        }
        if !columns.contains(&"transcript_path".to_string()) {
            self.conn.execute("ALTER TABLE sessions ADD COLUMN transcript_path TEXT", [])?;
        }
        if !columns.contains(&"git_branch".to_string()) {
            self.conn.execute("ALTER TABLE sessions ADD COLUMN git_branch TEXT", [])?;
        }
        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn upsert_session(
        &self,
        session_id: &str,
        project_path: &str,
        transcript_path: Option<&str>,
        git_branch: Option<&str>,
        status: &str,
        event_type: &str,
        tool_name: Option<&str>,
        now: &str,
    ) -> Result<bool> {
        let existing: Option<String> = self.conn
            .query_row(
                "SELECT session_id FROM sessions WHERE session_id = ?",
                [session_id],
                |row| row.get(0),
            )
            .ok();

        if existing.is_none() {
            let placeholder_id = format!("pending:{}", project_path);
            self.conn.execute("DELETE FROM sessions WHERE session_id = ?", [&placeholder_id])?;
            self.conn.execute(
                "INSERT INTO sessions (session_id, project_path, transcript_path, git_branch, status, last_event_type, last_tool, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![session_id, project_path, transcript_path, git_branch, status, event_type, tool_name, now, now],
            )?;
            Ok(false)
        } else {
            self.conn.execute(
                "UPDATE sessions SET status = ?, last_event_type = ?, last_tool = COALESCE(?, last_tool), transcript_path = COALESCE(?, transcript_path), git_branch = COALESCE(?, git_branch), updated_at = ? WHERE session_id = ?",
                params![status, event_type, tool_name, transcript_path, git_branch, now, session_id],
            )?;
            Ok(true)
        }
    }

    pub fn insert_event(&self, session_id: &str, event_type: &str, payload: &str, now: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events (session_id, event_type, payload, created_at) VALUES (?, ?, ?, ?)",
            params![session_id, event_type, payload, now],
        )?;
        Ok(())
    }

    pub fn upsert_todos(&self, session_id: &str, todos_json: &str, now: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO todos (session_id, todos_json, updated_at) VALUES (?, ?, ?)",
            params![session_id, todos_json, now],
        )?;
        Ok(())
    }

    pub fn get_sessions(&self) -> Result<Vec<crate::models::Session>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT s.session_id, s.project_path, s.git_branch, s.status,
                      s.last_event_type, s.last_tool, s.summary, s.created_at, s.updated_at,
                      s.transcript_path, t.todos_json
               FROM sessions s
               LEFT JOIN todos t ON s.session_id = t.session_id
               ORDER BY s.updated_at DESC"#
        )?;

        let rows = stmt.query_map([], |row| {
            let todos_json: Option<String> = row.get(10)?;
            let todo_progress = todos_json.and_then(|json| {
                serde_json::from_str::<Vec<serde_json::Value>>(&json).ok().map(|todos| {
                    let completed = todos.iter().filter(|t| t.get("status").and_then(|s| s.as_str()) == Some("completed")).count();
                    format!("{}/{}", completed, todos.len())
                })
            });

            let status_str: String = row.get(3)?;
            let status = status_str.parse().unwrap_or_default();

            Ok(crate::models::Session {
                session_id: row.get(0)?,
                project_path: row.get(1)?,
                git_branch: row.get(2)?,
                status,
                last_event_type: row.get(4)?,
                last_tool: row.get(5)?,
                summary: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                transcript_path: row.get(9)?,
                todo_progress,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn get_transcript_path(&self, session_id: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT transcript_path FROM sessions WHERE session_id = ?",
                [session_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn get_summary(&self, session_id: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT summary FROM sessions WHERE session_id = ?",
                [session_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn set_summary(&self, session_id: &str, summary: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET summary = ? WHERE session_id = ?",
            params![summary, session_id],
        )?;
        Ok(())
    }

    pub fn get_event_count(&self, session_id: &str) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE session_id = ?",
                [session_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn register_placeholder(&self, project_path: &str, now: &str) -> Result<String> {
        let placeholder_id = format!("pending:{}", project_path);
        let existing: Option<String> = self.conn
            .query_row(
                "SELECT session_id FROM sessions WHERE session_id = ?",
                [&placeholder_id],
                |row| row.get(0),
            )
            .ok();

        if existing.is_none() {
            self.conn.execute(
                "INSERT INTO sessions (session_id, project_path, status, created_at, updated_at) VALUES (?, ?, 'starting', ?, ?)",
                params![placeholder_id, project_path, now, now],
            )?;
        }
        Ok(placeholder_id)
    }

    pub fn mark_stopped(&self, project_path: &str, now: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id FROM sessions WHERE project_path = ? AND status NOT IN ('closed', 'error')"
        )?;
        let session_ids: Vec<String> = stmt
            .query_map([project_path], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        for session_id in &session_ids {
            self.conn.execute(
                "UPDATE sessions SET status = 'closed', updated_at = ? WHERE session_id = ?",
                params![now, session_id],
            )?;
        }
        Ok(session_ids)
    }

    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM todos WHERE session_id = ?", [session_id])?;
        self.conn.execute("DELETE FROM events WHERE session_id = ?", [session_id])?;
        self.conn.execute("DELETE FROM sessions WHERE session_id = ?", [session_id])?;
        Ok(())
    }
}
```

**Step 2: Update lib.rs**

```rust
pub mod config;
pub mod db;
pub mod models;
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/db.rs src/lib.rs
git commit -m "Add database layer"
```

---

## Task 5: Daemon - Core Server

**Files:**
- Create: `src/daemon/mod.rs`
- Create: `src/daemon/state.rs`
- Create: `src/daemon/handlers.rs`
- Create: `src/daemon/websocket.rs`

**Step 1: Create src/daemon/mod.rs**

```rust
pub mod handlers;
pub mod state;
pub mod websocket;

use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use state::AppState;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/status", get(handlers::status))
        .route("/events", post(handlers::post_event))
        .route("/sessions", get(handlers::get_sessions))
        .route("/sessions/register", post(handlers::register_session))
        .route("/sessions/stopped", post(handlers::mark_stopped))
        .route("/sessions/{session_id}", delete(handlers::delete_session))
        .route("/ws", get(websocket::websocket_handler))
        .with_state(state)
}

pub async fn run(state: Arc<AppState>, port: u16) -> anyhow::Result<()> {
    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

**Step 2: Create src/daemon/state.rs**

```rust
use crate::db::Database;
use std::sync::Mutex;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum WsMessage {
    SessionUpdate(String),
    SessionDeleted(String),
}

pub struct AppState {
    pub db: Mutex<Database>,
    pub tx: broadcast::Sender<WsMessage>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            db: Mutex::new(db),
            tx,
        }
    }
}
```

**Step 3: Create src/daemon/handlers.rs**

```rust
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::get_data_dir;
use crate::models::{HookEvent, SessionStatus};
use super::state::{AppState, WsMessage};

#[derive(Serialize)]
pub struct StatusResponse {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key_configured: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hooks_installed: Option<bool>,
}

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

pub async fn status() -> Json<StatusResponse> {
    let api_key_configured = std::env::var("ANTHROPIC_API_KEY").is_ok();
    Json(StatusResponse {
        status: "ok".to_string(),
        api_key_configured: Some(api_key_configured),
        hooks_installed: Some(true), // Hooks are built-in now
    })
}

fn determine_status(event: &HookEvent) -> SessionStatus {
    if event.event_type == "stop" {
        return if event.error.is_some() {
            SessionStatus::Error
        } else {
            SessionStatus::Closed
        };
    }
    if event.event_type == "session_start" || event.event_type == "tool_start" {
        return SessionStatus::Running;
    }
    SessionStatus::from_tool(event.tool_name.as_deref().unwrap_or(""))
}

pub async fn post_event(
    State(state): State<Arc<AppState>>,
    Json(event): Json<HookEvent>,
) -> Json<serde_json::Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let status = determine_status(&event);

    {
        let db = state.db.lock().unwrap();
        let _ = db.upsert_session(
            &event.session_id,
            &event.project_path,
            event.transcript_path.as_deref(),
            event.git_branch.as_deref(),
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

    let _ = state.tx.send(WsMessage::SessionUpdate(event.session_id.clone()));
    Json(serde_json::json!({"status": "ok"}))
}

pub async fn get_sessions(State(state): State<Arc<AppState>>) -> Json<Vec<crate::models::Session>> {
    let db = state.db.lock().unwrap();
    Json(db.get_sessions().unwrap_or_default())
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    project_path: String,
}

pub async fn register_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Json<serde_json::Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let placeholder_id = {
        let db = state.db.lock().unwrap();
        db.register_placeholder(&req.project_path, &now).unwrap_or_default()
    };
    let _ = state.tx.send(WsMessage::SessionUpdate(placeholder_id.clone()));
    Json(serde_json::json!({"status": "ok", "placeholder_id": placeholder_id}))
}

pub async fn mark_stopped(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Json<serde_json::Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let session_ids = {
        let db = state.db.lock().unwrap();
        db.mark_stopped(&req.project_path, &now).unwrap_or_default()
    };
    for session_id in &session_ids {
        let _ = state.tx.send(WsMessage::SessionUpdate(session_id.clone()));
    }
    Json(serde_json::json!({"status": "ok", "count": session_ids.len()}))
}

pub async fn delete_session(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Json<serde_json::Value> {
    {
        let db = state.db.lock().unwrap();
        let _ = db.delete_session(&session_id);
    }
    let _ = state.tx.send(WsMessage::SessionDeleted(session_id));
    Json(serde_json::json!({"status": "ok"}))
}
```

**Step 4: Create src/daemon/websocket.rs**

```rust
use axum::{
    extract::{State, WebSocketUpgrade},
    response::Response,
};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;

use super::state::{AppState, WsMessage};

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let json = match msg {
                WsMessage::SessionUpdate(id) => {
                    serde_json::json!({"type": "session_update", "session_id": id})
                }
                WsMessage::SessionDeleted(id) => {
                    serde_json::json!({"type": "session_deleted", "session_id": id})
                }
            };
            if sender.send(Message::Text(json.to_string().into())).await.is_err() {
                break;
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(_)) = receiver.next().await {}
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}
```

**Step 5: Update lib.rs**

```rust
pub mod config;
pub mod daemon;
pub mod db;
pub mod models;
```

**Step 6: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 7: Commit**

```bash
git add src/daemon/ src/lib.rs
git commit -m "Add daemon with Axum server"
```

---

## Task 6: Hook Subcommands

**Files:**
- Create: `src/hook/mod.rs`
- Create: `src/hook/session_start.rs`
- Create: `src/hook/tool_use.rs`
- Create: `src/hook/stop.rs`

**Step 1: Create src/hook/mod.rs**

```rust
pub mod session_start;
pub mod stop;
pub mod tool_use;

pub use session_start::session_start;
pub use stop::stop;
pub use tool_use::{pre_tool_use, post_tool_use};
```

**Step 2: Create src/hook/session_start.rs**

```rust
use anyhow::Result;
use std::io::Read;
use std::process::Command;

const DAEMON_URL: &str = "http://localhost:7890";

pub async fn session_start() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;

    let client = reqwest::Client::new();
    if client
        .get(format!("{}/health", DAEMON_URL))
        .timeout(std::time::Duration::from_secs(1))
        .send()
        .await
        .is_err()
    {
        return Ok(());
    }

    let data: serde_json::Value = serde_json::from_str(&input)?;
    let cwd = data["cwd"].as_str().unwrap_or(".");
    let git_branch = get_git_branch(cwd);

    let payload = serde_json::json!({
        "session_id": data["session_id"],
        "project_path": data["cwd"],
        "event_type": "session_start",
        "transcript_path": data["transcript_path"],
        "git_branch": git_branch,
    });

    let _ = client
        .post(format!("{}/events", DAEMON_URL))
        .json(&payload)
        .send()
        .await;

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
```

**Step 3: Create src/hook/tool_use.rs**

```rust
use anyhow::Result;
use std::io::Read;

const DAEMON_URL: &str = "http://localhost:7890";

pub async fn pre_tool_use() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;

    let client = reqwest::Client::new();
    if client
        .get(format!("{}/health", DAEMON_URL))
        .timeout(std::time::Duration::from_secs(1))
        .send()
        .await
        .is_err()
    {
        return Ok(());
    }

    let data: serde_json::Value = serde_json::from_str(&input)?;

    let payload = serde_json::json!({
        "session_id": data["session_id"],
        "project_path": data["cwd"],
        "event_type": "tool_start",
        "tool_name": data["tool_name"],
        "tool_params": data["tool_input"],
    });

    let _ = client
        .post(format!("{}/events", DAEMON_URL))
        .json(&payload)
        .send()
        .await;

    Ok(())
}

pub async fn post_tool_use() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;

    let client = reqwest::Client::new();
    if client
        .get(format!("{}/health", DAEMON_URL))
        .timeout(std::time::Duration::from_secs(1))
        .send()
        .await
        .is_err()
    {
        return Ok(());
    }

    let data: serde_json::Value = serde_json::from_str(&input)?;

    let payload = serde_json::json!({
        "session_id": data["session_id"],
        "project_path": data["cwd"],
        "event_type": "tool_call",
        "tool_name": data["tool_name"],
        "tool_params": data["tool_input"],
    });

    let _ = client
        .post(format!("{}/events", DAEMON_URL))
        .json(&payload)
        .send()
        .await;

    Ok(())
}
```

**Step 4: Create src/hook/stop.rs**

```rust
use anyhow::Result;
use std::io::Read;

const DAEMON_URL: &str = "http://localhost:7890";

pub async fn stop() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;

    let client = reqwest::Client::new();
    if client
        .get(format!("{}/health", DAEMON_URL))
        .timeout(std::time::Duration::from_secs(1))
        .send()
        .await
        .is_err()
    {
        return Ok(());
    }

    let data: serde_json::Value = serde_json::from_str(&input)?;

    let payload = serde_json::json!({
        "session_id": data["session_id"],
        "project_path": data["cwd"],
        "event_type": "stop",
        "error": data.get("stop_hook_active").and_then(|v| v.as_bool()).filter(|&b| !b).map(|_| "stopped"),
    });

    let _ = client
        .post(format!("{}/events", DAEMON_URL))
        .json(&payload)
        .send()
        .await;

    Ok(())
}
```

**Step 5: Update lib.rs**

```rust
pub mod config;
pub mod daemon;
pub mod db;
pub mod hook;
pub mod models;
```

**Step 6: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 7: Commit**

```bash
git add src/hook/ src/lib.rs
git commit -m "Add hook subcommands"
```

---

## Task 7: CLI Commands

**Files:**
- Create: `src/cli/mod.rs`
- Create: `src/cli/run.rs`
- Create: `src/cli/install.rs`
- Create: `src/cli/sessions.rs`
- Create: `src/cli/kill.rs`
- Create: `src/cli/config.rs`

**Step 1: Create src/cli/mod.rs**

```rust
pub mod config;
pub mod install;
pub mod kill;
pub mod run;
pub mod sessions;
```

**Step 2: Create src/cli/run.rs**

```rust
use anyhow::Result;
use std::os::unix::process::CommandExt;
use std::process::Command;

const DAEMON_URL: &str = "http://localhost:7890";

pub fn run(claude_args: Vec<String>) -> Result<()> {
    ensure_daemon_running();

    let project_path = std::env::current_dir()?.to_string_lossy().to_string();

    unsafe {
        let pid = libc::fork();
        if pid == 0 {
            let err = Command::new("claude").args(&claude_args).exec();
            eprintln!("Failed to exec claude: {}", err);
            std::process::exit(1);
        } else if pid > 0 {
            setup_signal_handlers(pid, project_path.clone());

            let mut status: i32 = 0;
            libc::waitpid(pid, &mut status, 0);
            notify_stopped(&project_path);
        } else {
            anyhow::bail!("Fork failed");
        }
    }

    Ok(())
}

fn ensure_daemon_running() {
    let client = reqwest::blocking::Client::new();
    if client
        .get(format!("{}/health", DAEMON_URL))
        .timeout(std::time::Duration::from_secs(1))
        .send()
        .is_ok()
    {
        return;
    }

    let exe = std::env::current_exe().unwrap();
    Command::new(&exe)
        .arg("daemon")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok();

    std::thread::sleep(std::time::Duration::from_secs(1));
}

fn notify_stopped(project_path: &str) {
    let client = reqwest::blocking::Client::new();
    let _ = client
        .post(format!("{}/sessions/stopped", DAEMON_URL))
        .json(&serde_json::json!({"project_path": project_path}))
        .timeout(std::time::Duration::from_secs(2))
        .send();
}

static mut CHILD_PID: i32 = 0;
static mut PROJECT_PATH: Option<String> = None;

fn setup_signal_handlers(pid: i32, project_path: String) {
    unsafe {
        CHILD_PID = pid;
        PROJECT_PATH = Some(project_path);

        libc::signal(libc::SIGHUP, handle_signal as usize);
        libc::signal(libc::SIGTERM, handle_signal as usize);
        libc::signal(libc::SIGINT, handle_signal as usize);
    }
}

extern "C" fn handle_signal(_sig: i32) {
    unsafe {
        if CHILD_PID > 0 {
            libc::kill(CHILD_PID, libc::SIGTERM);
        }
        if let Some(ref path) = PROJECT_PATH {
            notify_stopped(path);
        }
        std::process::exit(0);
    }
}
```

**Step 3: Create src/cli/install.rs**

```rust
use anyhow::Result;

pub fn install() -> Result<()> {
    println!("Add to ~/.claude/settings.json:");
    let config = serde_json::json!({
        "hooks": {
            "SessionStart": [{
                "hooks": [{
                    "type": "command",
                    "command": "sp hook session-start"
                }]
            }],
            "PreToolUse": [{
                "matcher": "*",
                "hooks": [{
                    "type": "command",
                    "command": "sp hook pre-tool-use"
                }]
            }],
            "PostToolUse": [{
                "matcher": "*",
                "hooks": [{
                    "type": "command",
                    "command": "sp hook post-tool-use"
                }]
            }],
            "Stop": [{
                "hooks": [{
                    "type": "command",
                    "command": "sp hook stop"
                }]
            }]
        }
    });
    println!("{}", serde_json::to_string_pretty(&config)?);
    Ok(())
}
```

**Step 4: Create src/cli/sessions.rs**

```rust
use anyhow::Result;

const DAEMON_URL: &str = "http://localhost:7890";

pub fn sessions() -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(format!("{}/sessions", DAEMON_URL))
        .timeout(std::time::Duration::from_secs(5))
        .send()?;
    let sessions: serde_json::Value = response.json()?;
    println!("{}", serde_json::to_string_pretty(&sessions)?);
    Ok(())
}
```

**Step 5: Create src/cli/kill.rs**

```rust
use anyhow::Result;
use std::process::Command;

pub fn kill() -> Result<()> {
    let patterns = ["plate-spinner.*daemon", "sp daemon"];
    let mut killed = false;

    for pattern in patterns {
        let result = Command::new("pkill").arg("-f").arg(pattern).status();
        if result.map(|s| s.success()).unwrap_or(false) {
            killed = true;
        }
    }

    if killed {
        println!("Daemon stopped");
    } else {
        println!("No daemon running");
    }
    Ok(())
}
```

**Step 6: Create src/cli/config.rs**

```rust
use anyhow::Result;
use crate::config::{get_config_path, load_config, save_config, Config};

pub fn config_path() -> Result<()> {
    println!("{}", get_config_path().display());
    Ok(())
}

pub fn config_export() -> Result<()> {
    let config = load_config();
    let path = get_config_path();
    if path.exists() {
        print!("{}", std::fs::read_to_string(&path)?);
    } else {
        save_config(&config)?;
        print!("{}", std::fs::read_to_string(&path)?);
    }
    Ok(())
}

pub fn config_import(file: &str) -> Result<()> {
    let content = std::fs::read_to_string(file)?;
    let config: Config = toml::from_str(&content)?;
    save_config(&config)?;
    println!("Imported config from {}", file);
    Ok(())
}
```

**Step 7: Update lib.rs**

```rust
pub mod cli;
pub mod config;
pub mod daemon;
pub mod db;
pub mod hook;
pub mod models;
```

**Step 8: Add libc and reqwest blocking to Cargo.toml**

Add to `[dependencies]`:
```toml
libc = "0.2"
```

And modify reqwest:
```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls", "blocking"], default-features = false }
```

**Step 9: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 10: Commit**

```bash
git add Cargo.toml src/cli/ src/lib.rs
git commit -m "Add CLI commands"
```

---

## Task 8: TUI - Core App

**Files:**
- Create: `src/tui/mod.rs`
- Create: `src/tui/app.rs`
- Create: `src/tui/state.rs`
- Create: `src/tui/ui.rs`

**Step 1: Create src/tui/mod.rs**

```rust
pub mod app;
pub mod state;
pub mod ui;

pub use app::run;
```

**Step 2: Create src/tui/state.rs**

```rust
use std::collections::HashSet;
use crate::config::Config;
use crate::models::{Session, SessionStatus};

pub struct App {
    pub sessions: Vec<Session>,
    pub selected_index: usize,
    pub seen_sessions: HashSet<String>,
    pub previous_statuses: std::collections::HashMap<String, SessionStatus>,
    pub config: Config,
    pub should_quit: bool,
    pub resume_session: Option<(String, String)>,
    pub show_sound_settings: bool,
    pub sound_settings_row: usize,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            sessions: Vec::new(),
            selected_index: 0,
            seen_sessions: HashSet::new(),
            previous_statuses: std::collections::HashMap::new(),
            config,
            should_quit: false,
            resume_session: None,
            show_sound_settings: false,
            sound_settings_row: 0,
        }
    }

    pub fn display_order(&self) -> Vec<&Session> {
        let mut open: Vec<_> = self.sessions.iter().filter(|s| s.status != SessionStatus::Closed).collect();
        let closed: Vec<_> = self.sessions.iter().filter(|s| s.status == SessionStatus::Closed).collect();

        open.sort_by(|a, b| {
            let a_attention = a.status.needs_attention();
            let b_attention = b.status.needs_attention();
            b_attention.cmp(&a_attention)
        });

        open.into_iter().chain(closed).collect()
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let max = self.display_order().len().saturating_sub(1);
        if self.selected_index < max {
            self.selected_index += 1;
        }
    }

    pub fn select(&mut self) {
        let order = self.display_order();
        if let Some(session) = order.get(self.selected_index) {
            if session.status == SessionStatus::Starting {
                return;
            }
            self.resume_session = Some((session.session_id.clone(), session.project_path.clone()));
            self.should_quit = true;
        }
    }

    pub fn jump(&mut self, index: usize) {
        let order = self.display_order();
        if index > 0 && index <= order.len() {
            self.selected_index = index - 1;
            self.select();
        }
    }

    pub fn mark_seen(&mut self, session_id: &str) {
        self.seen_sessions.insert(session_id.to_string());
    }

    pub fn is_unseen(&self, session: &Session) -> bool {
        session.status.needs_attention() && !self.seen_sessions.contains(&session.session_id)
    }

    pub fn attention_count(&self) -> usize {
        self.sessions.iter().filter(|s| s.status.needs_attention() && s.status != SessionStatus::Closed).count()
    }
}
```

**Step 3: Create src/tui/ui.rs**

```rust
use ratatui::{
    prelude::*,
    widgets::*,
};
use crate::models::SessionStatus;
use super::state::App;

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    render_header(f, chunks[0], app);
    render_sessions(f, chunks[1], app);
    render_footer(f, chunks[2]);

    if app.show_sound_settings {
        render_sound_settings(f, app);
    }
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let attention = app.attention_count();
    let title = if attention > 0 {
        format!(" Plate-Spinner ({} need attention) ", attention)
    } else {
        " Plate-Spinner ".to_string()
    };
    let header = Paragraph::new(title).style(Style::default().bold());
    f.render_widget(header, area);
}

fn render_sessions(f: &mut Frame, area: Rect, app: &App) {
    let order = app.display_order();

    if order.is_empty() {
        let msg = "No active sessions.\n\nRun 'sp run' to start a tracked session.";
        let para = Paragraph::new(msg).alignment(Alignment::Center);
        f.render_widget(para, area);
        return;
    }

    let mut lines = Vec::new();
    let mut in_closed = false;
    let index_width = order.len().to_string().len();

    for (i, session) in order.iter().enumerate() {
        if !in_closed && session.status == SessionStatus::Closed {
            in_closed = true;
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("CLOSED", Style::default().dim().bold())));
        } else if i == 0 {
            lines.push(Line::from(Span::styled("OPEN", Style::default().dim().bold())));
        }

        let line = format_session_line(session, i + 1, index_width, app.is_unseen(session), i == app.selected_index);
        lines.push(line);
    }

    let para = Paragraph::new(lines);
    f.render_widget(para, area);
}

fn format_session_line(session: &crate::models::Session, index: usize, index_width: usize, unseen: bool, selected: bool) -> Line<'static> {
    let status = session.status;
    let folder = session.project_name();
    let branch = session.git_branch.as_deref().unwrap_or("");
    let todo = session.todo_progress.as_deref().unwrap_or("");
    let summary = session.summary.as_deref().unwrap_or("");

    let label = if !branch.is_empty() {
        let full = format!("{}/{}", folder, branch);
        if full.len() > 20 {
            format!("{}...", &full[..17])
        } else {
            full
        }
    } else if folder.len() > 20 {
        format!("{}...", &folder[..17])
    } else {
        folder.to_string()
    };

    let icon = status.icon();
    let status_short = status.short_name();
    let color = status_color(status);

    let unseen_marker = if unseen { "*" } else { " " };

    let mut spans = vec![
        Span::raw(format!("[{:>width$}]", index, width = index_width)),
        Span::raw(unseen_marker),
        Span::styled(format!("{}", icon), Style::default().fg(color)),
        Span::raw(" "),
        Span::raw(format!("{:<20}", label)),
        Span::raw(" "),
        Span::styled(format!("{:<7}", status_short), Style::default().fg(color)),
    ];

    if !todo.is_empty() {
        spans.push(Span::raw(format!(" [{}]", todo)));
    }
    if !summary.is_empty() {
        spans.push(Span::raw(format!(" {}", summary)));
    }

    let style = if selected {
        Style::default().bg(Color::DarkGray)
    } else {
        Style::default()
    };

    Line::from(spans).style(style)
}

fn status_color(status: SessionStatus) -> Color {
    match status {
        SessionStatus::Starting => Color::DarkGray,
        SessionStatus::Running => Color::Green,
        SessionStatus::Idle => Color::Cyan,
        SessionStatus::AwaitingInput => Color::Yellow,
        SessionStatus::AwaitingApproval => Color::Magenta,
        SessionStatus::Error => Color::Red,
        SessionStatus::Closed => Color::DarkGray,
    }
}

fn render_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(" q:Quit  r:Refresh  s:Sounds  Enter:Resume  Del:Dismiss ")
        .style(Style::default().dim());
    f.render_widget(footer, area);
}

fn render_sound_settings(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 60, f.area());
    let block = Block::default()
        .title(" Sound Settings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    f.render_widget(Clear, area);
    f.render_widget(block.clone(), area);

    let inner = block.inner(area);
    let settings = [
        ("Enabled", if app.config.sounds.enabled { "yes" } else { "no" }),
        ("Awaiting Input", &app.config.sounds.awaiting_input),
        ("Awaiting Approval", &app.config.sounds.awaiting_approval),
        ("Error", &app.config.sounds.error),
        ("Idle", &app.config.sounds.idle),
        ("Closed", &app.config.sounds.closed),
    ];

    let lines: Vec<Line> = settings
        .iter()
        .enumerate()
        .map(|(i, (label, value))| {
            let style = if i == app.sound_settings_row {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            Line::from(format!(" {:<20} < {} >", label, value)).style(style)
        })
        .collect();

    let para = Paragraph::new(lines);
    f.render_widget(para, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
```

**Step 4: Create src/tui/app.rs**

```rust
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::{load_config, play_sound, save_config, AVAILABLE_SOUNDS};
use crate::models::SessionStatus;
use super::state::App;
use super::ui;

const DAEMON_URL: &str = "http://localhost:7890";

pub async fn run() -> Result<Option<(String, String)>> {
    let config = load_config();
    let mut app = App::new(config);

    let mut terminal = ratatui::init();

    let (tx, mut rx) = mpsc::channel::<()>(32);
    tokio::spawn(connect_websocket(tx));

    refresh(&mut app).await;

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                if event::poll(Duration::ZERO)? {
                    if let Event::Key(key) = event::read()? {
                        if key.kind == KeyEventKind::Press {
                            handle_key(&mut app, key.code).await;
                        }
                    }
                }
            }
            _ = rx.recv() => {
                refresh(&mut app).await;
            }
        }

        if app.should_quit {
            break;
        }
    }

    ratatui::restore();
    Ok(app.resume_session)
}

async fn handle_key(app: &mut App, key: KeyCode) {
    if app.show_sound_settings {
        handle_sound_settings_key(app, key);
        return;
    }

    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('r') => refresh(app).await,
        KeyCode::Char('s') => app.show_sound_settings = true,
        KeyCode::Up => app.move_up(),
        KeyCode::Down => app.move_down(),
        KeyCode::Enter => app.select(),
        KeyCode::Delete | KeyCode::Backspace => dismiss(app).await,
        KeyCode::Char(c @ '1'..='9') => app.jump(c.to_digit(10).unwrap() as usize),
        _ => {}
    }

    let order = app.display_order();
    if let Some(session) = order.get(app.selected_index) {
        app.mark_seen(&session.session_id);
    }
}

fn handle_sound_settings_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.show_sound_settings = false,
        KeyCode::Up => {
            if app.sound_settings_row > 0 {
                app.sound_settings_row -= 1;
            }
        }
        KeyCode::Down => {
            if app.sound_settings_row < 5 {
                app.sound_settings_row += 1;
            }
        }
        KeyCode::Left | KeyCode::Right => {
            let direction = if key == KeyCode::Right { 1i32 } else { -1i32 };
            change_sound_setting(app, direction);
        }
        KeyCode::Enter => {
            change_sound_setting(app, 1);
        }
        _ => {}
    }
}

fn change_sound_setting(app: &mut App, direction: i32) {
    match app.sound_settings_row {
        0 => {
            app.config.sounds.enabled = !app.config.sounds.enabled;
        }
        row => {
            let current = match row {
                1 => &app.config.sounds.awaiting_input,
                2 => &app.config.sounds.awaiting_approval,
                3 => &app.config.sounds.error,
                4 => &app.config.sounds.idle,
                5 => &app.config.sounds.closed,
                _ => return,
            };
            let idx = AVAILABLE_SOUNDS.iter().position(|&s| s == current).unwrap_or(0);
            let new_idx = (idx as i32 + direction).rem_euclid(AVAILABLE_SOUNDS.len() as i32) as usize;
            let new_sound = AVAILABLE_SOUNDS[new_idx].to_string();

            if new_sound != "none" {
                play_sound(&new_sound);
            }

            match row {
                1 => app.config.sounds.awaiting_input = new_sound,
                2 => app.config.sounds.awaiting_approval = new_sound,
                3 => app.config.sounds.error = new_sound,
                4 => app.config.sounds.idle = new_sound,
                5 => app.config.sounds.closed = new_sound,
                _ => {}
            }
        }
    }
    let _ = save_config(&app.config);
}

async fn refresh(app: &mut App) {
    let client = reqwest::Client::new();
    if let Ok(response) = client
        .get(format!("{}/sessions", DAEMON_URL))
        .send()
        .await
    {
        if let Ok(sessions) = response.json().await {
            for session in &sessions {
                let session: &crate::models::Session = session;
                let prev = app.previous_statuses.get(&session.session_id);

                if prev == Some(&SessionStatus::Running) && session.status != SessionStatus::Running {
                    if app.config.sounds.enabled {
                        let sound = match session.status {
                            SessionStatus::AwaitingInput => &app.config.sounds.awaiting_input,
                            SessionStatus::AwaitingApproval => &app.config.sounds.awaiting_approval,
                            SessionStatus::Error => &app.config.sounds.error,
                            SessionStatus::Idle => &app.config.sounds.idle,
                            SessionStatus::Closed => &app.config.sounds.closed,
                            _ => "none",
                        };
                        play_sound(sound);
                    }
                }

                let prev_attention = prev.map(|s| s.needs_attention()).unwrap_or(false);
                let curr_attention = session.status.needs_attention();
                if curr_attention && !prev_attention {
                    app.seen_sessions.remove(&session.session_id);
                }

                app.previous_statuses.insert(session.session_id.clone(), session.status);
            }
            app.sessions = sessions;
        }
    }
}

async fn dismiss(app: &mut App) {
    let order = app.display_order();
    if let Some(session) = order.get(app.selected_index) {
        let session_id = session.session_id.clone();
        let client = reqwest::Client::new();
        let _ = client
            .delete(format!("{}/sessions/{}", DAEMON_URL, session_id))
            .send()
            .await;

        if app.selected_index >= order.len().saturating_sub(1) {
            app.selected_index = app.selected_index.saturating_sub(1);
        }
        refresh(app).await;
    }
}

async fn connect_websocket(tx: mpsc::Sender<()>) {
    use tokio_tungstenite::connect_async;
    use futures_util::StreamExt;

    loop {
        if let Ok((mut ws, _)) = connect_async("ws://localhost:7890/ws").await {
            while let Some(Ok(_)) = ws.next().await {
                let _ = tx.send(()).await;
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
```

**Step 5: Update lib.rs**

```rust
pub mod cli;
pub mod config;
pub mod daemon;
pub mod db;
pub mod hook;
pub mod models;
pub mod tui;
```

**Step 6: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 7: Commit**

```bash
git add src/tui/ src/lib.rs
git commit -m "Add TUI with Ratatui"
```

---

## Task 9: Wire Up main.rs

**Files:**
- Modify: `src/main.rs`

**Step 1: Replace main.rs with full implementation**

```rust
use clap::{Parser, Subcommand};
use std::sync::Arc;

use plate_spinner::config::get_data_dir;
use plate_spinner::daemon::state::AppState;
use plate_spinner::db::Database;

#[derive(Parser)]
#[command(name = "sp", about = "Dashboard for managing Claude Code sessions")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Run daemon in foreground")]
    Daemon,
    #[command(about = "Launch Claude with tracking")]
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        claude_args: Vec<String>,
    },
    #[command(about = "List sessions as JSON")]
    Sessions,
    #[command(about = "Install hooks")]
    Install,
    #[command(about = "Stop the daemon")]
    Kill,
    #[command(about = "Manage configuration")]
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommands>,
    },
    #[command(about = "Hook handlers (called by Claude Code)")]
    Hook {
        #[command(subcommand)]
        hook_type: HookCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    #[command(about = "Print config file path")]
    Path,
    #[command(about = "Export config to stdout")]
    Export,
    #[command(about = "Import config from file")]
    Import { file: String },
}

#[derive(Subcommand)]
enum HookCommands {
    #[command(name = "session-start")]
    SessionStart,
    #[command(name = "pre-tool-use")]
    PreToolUse,
    #[command(name = "post-tool-use")]
    PostToolUse,
    Stop,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Daemon) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                let db_path = get_data_dir().join("state.db");
                let db = Database::open(&db_path)?;
                let state = Arc::new(AppState::new(db));
                plate_spinner::daemon::run(state, 7890).await
            })
        }
        Some(Commands::Run { claude_args }) => {
            plate_spinner::cli::run::run(claude_args)
        }
        Some(Commands::Sessions) => {
            plate_spinner::cli::sessions::sessions()
        }
        Some(Commands::Install) => {
            plate_spinner::cli::install::install()
        }
        Some(Commands::Kill) => {
            plate_spinner::cli::kill::kill()
        }
        Some(Commands::Config { command }) => {
            match command {
                Some(ConfigCommands::Path) => plate_spinner::cli::config::config_path(),
                Some(ConfigCommands::Export) => plate_spinner::cli::config::config_export(),
                Some(ConfigCommands::Import { file }) => plate_spinner::cli::config::config_import(&file),
                None => plate_spinner::cli::config::config_path(),
            }
        }
        Some(Commands::Hook { hook_type }) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                match hook_type {
                    HookCommands::SessionStart => plate_spinner::hook::session_start().await,
                    HookCommands::PreToolUse => plate_spinner::hook::pre_tool_use().await,
                    HookCommands::PostToolUse => plate_spinner::hook::post_tool_use().await,
                    HookCommands::Stop => plate_spinner::hook::stop().await,
                }
            })
        }
        None => {
            ensure_daemon_running();
            let rt = tokio::runtime::Runtime::new()?;
            let result = rt.block_on(plate_spinner::tui::run())?;

            if let Some((session_id, project_path)) = result {
                std::env::set_current_dir(&project_path)?;
                let err = std::process::Command::new("claude")
                    .args(["--resume", &session_id])
                    .exec();
                eprintln!("Failed to exec claude: {}", err);
            }
            Ok(())
        }
    }
}

fn ensure_daemon_running() {
    let client = reqwest::blocking::Client::new();
    if client
        .get("http://localhost:7890/health")
        .timeout(std::time::Duration::from_secs(1))
        .send()
        .is_ok()
    {
        return;
    }

    let exe = std::env::current_exe().unwrap();
    std::process::Command::new(&exe)
        .arg("daemon")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok();

    std::thread::sleep(std::time::Duration::from_secs(1));
}

use std::os::unix::process::CommandExt;
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 3: Test basic commands**

Run: `cargo run -- --help`
Expected: Shows help with all subcommands

Run: `cargo run -- install`
Expected: Prints JSON hook configuration

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "Wire up main.rs with all commands"
```

---

## Task 10: Update README and Clean Up

**Files:**
- Modify: `README.md`
- Delete: `plugin/` directory (no longer needed)

**Step 1: Update README.md**

```markdown
# Plate-Spinner

Dashboard for managing multiple concurrent Claude Code sessions.

## Quick Start

```bash
cargo install --path .
sp install              # Prints hook config to add to ~/.claude/settings.json
sp                      # Open dashboard (terminal 1)
sp run                  # Start tracked session (terminal 2)
sp run                  # Start another (terminal 3)
```

## Usage

**Dashboard** (`sp`): Shows all sessions in two groups:
- **OPEN**: Active sessions, sorted with "needs attention" first
- **CLOSED**: Sessions that have exited

Keybindings:
- `1-9` - Jump to session and resume
- `up/down` - Navigate sessions
- `enter` - Resume selected session
- `delete` - Dismiss selected session
- `s` - Sound settings
- `r` - Refresh
- `q` - Quit

## Session States

| Icon | Status | Trigger |
|------|--------|---------|
| `.` | starting | Session registered, no activity yet |
| `>` | running | Tool executing |
| `?` | awaiting_input | `AskUserQuestion` called |
| `!` | awaiting_approval | `ExitPlanMode` called |
| `-` | idle | Stop event received |
| `X` | error | Stop event with error |
| `x` | closed | Session wrapper exited |

AI summaries appear when sessions reach a waiting state (requires `ANTHROPIC_API_KEY`).

## Commands

```
sp              Dashboard (auto-starts daemon)
sp run [args]   Launch Claude with tracking
sp install      Print settings.json hook config
sp kill         Stop daemon
sp sessions     List sessions as JSON
sp daemon       Run daemon in foreground
sp config       Manage configuration
  path          Print config file path
  export        Export config to stdout
  import <file> Import config from file
```

## Architecture

```
Claude Code
    | hooks call `sp hook <type>`
    v
sp hook session-start/pre-tool-use/post-tool-use/stop
    | POST localhost:7890
    v
sp daemon (SQLite + WebSocket) --> sp (TUI)
```

## Building

```bash
cargo build --release
# Binary at target/release/sp
```

## Requirements

- Rust 1.70+
- Claude Code
- `ANTHROPIC_API_KEY` (optional, enables summaries)
```

**Step 2: Remove plugin directory**

```bash
rm -rf plugin/
```

**Step 3: Verify final build**

Run: `cargo build --release`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add -A
git commit -m "Update README for Rust, remove Python plugin"
```

---

## Task 11: Integration Testing

**Step 1: Test daemon startup**

```bash
cargo run -- daemon &
sleep 2
curl http://localhost:7890/health
# Expected: {"status":"ok"}
pkill -f "sp daemon"
```

**Step 2: Test hook commands**

```bash
echo '{"session_id":"test","cwd":"/tmp","transcript_path":null}' | cargo run -- hook session-start
# Expected: silent (daemon not running)
```

**Step 3: Test TUI launch**

```bash
cargo run
# Expected: TUI launches, shows "No active sessions"
# Press 'q' to quit
```

**Step 4: Test full flow**

```bash
# Terminal 1: Start dashboard
cargo run

# Terminal 2: Start a tracked session (requires claude CLI)
cargo run -- run

# Dashboard should show the session
# Press 'q' in dashboard to quit
```

**Step 5: Final commit if any fixes needed**

```bash
git add -A
git commit -m "Fix integration issues" # if needed
```

---

## Summary

After completing all tasks, you will have:
- A single `sp` binary (~5-10MB) with all functionality
- No Python dependencies
- Embedded sound files
- Full feature parity with the Python version
- Simpler hook configuration (no shell scripts)
