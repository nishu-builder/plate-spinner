# Plate-Spinner Rust Rewrite Design

## Goals

1. **Single binary distribution** - No Python runtime, no external dependencies
2. **Rust learning project** - Practice Rust with a real, useful application
3. **Full feature parity** - All existing functionality preserved

## Architecture Overview

```
plate-spinner (single binary: `sp`)
├── sp              → Launch TUI (starts daemon if needed)
├── sp run [args]   → Fork claude with tracking, notify daemon on exit
├── sp daemon       → Run HTTP server in foreground
├── sp install      → Print settings.json hook config
├── sp kill         → Stop daemon
├── sp sessions     → List sessions as JSON
├── sp config ...   → Config management
└── sp hook <type>  → Hook handlers (called by Claude Code)
    ├── session-start
    ├── pre-tool-use
    ├── post-tool-use
    └── stop

Data flow:
Claude Code → hooks in settings.json call `sp hook <type>`
           → HTTP POST to daemon (localhost:7890)
           → SQLite update + WebSocket broadcast
           → TUI receives update, re-renders
```

Shell scripts in `~/.plate-spinner/hooks/` are eliminated. The `sp install` command outputs settings.json config that calls `sp hook <type>` directly.

## Technology Choices

| Component | Library | Rationale |
|-----------|---------|-----------|
| CLI | clap | Standard, derive macros |
| HTTP Server | Axum | Tower-based, async, popular |
| Database | rusqlite (bundled) | SQLite, no external deps |
| TUI | Ratatui + crossterm | Most popular, good async |
| HTTP Client | reqwest | Standard choice |
| AI Summarizer | anthropic SDK | Official Rust SDK |
| Config | serde + toml | Standard for Rust |
| Audio | rodio | Cross-platform, simple |
| Async Runtime | tokio | Required by Axum |

## Project Structure

```
plate-spinner/
├── Cargo.toml
├── src/
│   ├── main.rs           # CLI entry point (clap)
│   ├── lib.rs            # Shared exports
│   ├── models.rs         # Session, HookEvent, SessionStatus
│   ├── config.rs         # TOML config, sound playback
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── run.rs        # sp run - fork/exec claude
│   │   ├── install.rs    # sp install
│   │   ├── sessions.rs   # sp sessions
│   │   ├── kill.rs       # sp kill
│   │   └── config.rs     # sp config subcommands
│   ├── hook/
│   │   ├── mod.rs        # sp hook dispatcher
│   │   ├── session_start.rs
│   │   ├── tool_use.rs   # pre and post
│   │   └── stop.rs
│   ├── daemon/
│   │   ├── mod.rs
│   │   ├── app.rs        # Axum router
│   │   ├── db.rs         # SQLite operations
│   │   ├── handlers.rs   # Route handlers
│   │   ├── websocket.rs  # WS connection manager
│   │   └── summarizer.rs # Anthropic API
│   └── tui/
│       ├── mod.rs
│       ├── app.rs        # Main loop
│       ├── ui.rs         # Rendering
│       ├── state.rs      # App state
│       └── sounds.rs     # Audio (uses config.rs)
├── sounds/               # .wav files (embedded)
└── tests/
```

## Data Models

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub session_id: String,
    pub project_path: String,
    pub event_type: String,
    pub tool_name: Option<String>,
    pub tool_params: Option<serde_json::Value>,
    pub transcript_path: Option<String>,
    pub git_branch: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub project_path: String,
    pub transcript_path: Option<String>,
    pub git_branch: Option<String>,
    pub status: SessionStatus,
    pub last_event_type: Option<String>,
    pub last_tool: Option<String>,
    pub summary: Option<String>,
    pub todo_progress: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

## Daemon (Axum)

```rust
use axum::{
    extract::{Path, State, WebSocketUpgrade},
    routing::{delete, get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct AppState {
    pub db: tokio::sync::Mutex<rusqlite::Connection>,
    pub tx: broadcast::Sender<WsMessage>,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/events", post(post_event))
        .route("/sessions", get(get_sessions))
        .route("/sessions/register", post(register_session))
        .route("/sessions/stopped", post(mark_stopped))
        .route("/sessions/{session_id}", delete(delete_session))
        .route("/ws", get(websocket_handler))
        .with_state(state)
}
```

Key patterns:
- `Arc<AppState>` for shared state
- `tokio::sync::Mutex` for async-safe DB access
- `broadcast::Sender` for WebSocket fan-out

## TUI (Ratatui)

```rust
use crossterm::event::{self, Event, KeyCode};
use ratatui::prelude::*;
use tokio::sync::mpsc;

pub struct App {
    sessions: Vec<Session>,
    selected_index: usize,
    seen_sessions: HashSet<String>,
    config: Config,
    should_quit: bool,
    resume_session: Option<(String, String)>,
}

pub async fn run(daemon_url: &str) -> anyhow::Result<Option<(String, String)>> {
    let mut terminal = ratatui::init();
    let mut app = App::new();

    let (tx, mut rx) = mpsc::channel::<()>(32);
    tokio::spawn(connect_websocket(daemon_url.to_string(), tx));

    app.refresh(daemon_url).await?;

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        tokio::select! {
            _ = poll_event() => {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => app.should_quit = true,
                        KeyCode::Char('r') => app.refresh(daemon_url).await?,
                        KeyCode::Up => app.move_up(),
                        KeyCode::Down => app.move_down(),
                        KeyCode::Enter => app.select(),
                        KeyCode::Char(c @ '1'..='9') => {
                            app.jump(c.to_digit(10).unwrap() as usize)
                        }
                        KeyCode::Delete | KeyCode::Backspace => {
                            app.dismiss(daemon_url).await?
                        }
                        KeyCode::Char('s') => app.show_sound_settings(),
                        _ => {}
                    }
                }
            }
            _ = rx.recv() => {
                app.refresh(daemon_url).await?;
            }
        }

        if app.should_quit {
            break;
        }
    }

    ratatui::restore();
    Ok(app.resume_session)
}
```

## Hooks

Hooks are subcommands that read JSON from stdin and POST to the daemon:

```rust
pub async fn session_start() -> Result<()> {
    let input = read_stdin()?;

    if Client::new()
        .get("http://localhost:7890/health")
        .timeout(Duration::from_secs(1))
        .send()
        .await
        .is_err()
    {
        return Ok(());
    }

    let data: serde_json::Value = serde_json::from_str(&input)?;
    let git_branch = get_git_branch(data["cwd"].as_str().unwrap_or("."));

    let payload = json!({
        "session_id": data["session_id"],
        "project_path": data["cwd"],
        "event_type": "session_start",
        "transcript_path": data["transcript_path"],
        "git_branch": git_branch,
    });

    let _ = Client::new()
        .post("http://localhost:7890/events")
        .json(&payload)
        .send()
        .await;

    Ok(())
}
```

The `sp install` command outputs settings.json config:

```json
{
  "hooks": {
    "SessionStart": [{"hooks": [{"type": "command", "command": "sp hook session-start"}]}],
    "PreToolUse": [{"matcher": "*", "hooks": [{"type": "command", "command": "sp hook pre-tool-use"}]}],
    "PostToolUse": [{"matcher": "*", "hooks": [{"type": "command", "command": "sp hook post-tool-use"}]}],
    "Stop": [{"hooks": [{"type": "command", "command": "sp hook stop"}]}]
  }
}
```

## Config and Sounds

Sounds are embedded at compile time:

```rust
pub const SOUND_BELL: &[u8] = include_bytes!("../sounds/bell.wav");
pub const SOUND_POP: &[u8] = include_bytes!("../sounds/pop.wav");

pub fn play_sound(name: &str) {
    let bytes = match name {
        "bell" => SOUND_BELL,
        "pop" => SOUND_POP,
        _ => return,
    };

    std::thread::spawn(move || {
        if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
            if let Ok(source) = rodio::Decoder::new(std::io::Cursor::new(bytes)) {
                let _ = handle.play_raw(source.convert_samples());
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    });
}
```

Config uses TOML at `~/.config/plate-spinner/config.toml`.

## Cargo.toml

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
reqwest = { version = "0.12", features = ["json"] }
rodio = "0.19"
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
toml = "0.8"

[profile.release]
lto = true
strip = true
```

## Installation

```bash
cargo install --path .
sp install  # prints settings.json config to add
sp          # launch dashboard
sp run      # start tracked session
```

## Implementation Order

1. Project scaffolding + Cargo.toml
2. Models + config
3. Database layer
4. Daemon (Axum server, all routes)
5. Hook subcommands
6. CLI commands (run, install, kill, sessions, config)
7. TUI (basic rendering)
8. TUI (WebSocket updates, keyboard handling)
9. TUI (sound settings modal)
10. Summarizer (Anthropic API)
11. Testing + polish
