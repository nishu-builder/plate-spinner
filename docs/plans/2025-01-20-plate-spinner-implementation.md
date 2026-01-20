# Plate-Spinner Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a dashboard for managing multiple concurrent Claude Code sessions with a TUI and tmux integration.

**Architecture:** Claude Code plugin captures tool events via hooks, POSTs to a local FastAPI daemon that persists to SQLite, and a Textual TUI displays sessions with real-time WebSocket updates.

**Tech Stack:** Python 3.11+, FastAPI, Textual, SQLite, httpx, uvicorn

---

## Phase 1: Discovery & Foundation

### Task 1: Discover Hook Environment Variables

**Goal:** Determine what env vars Claude Code exposes in hooks.

**Files:**
- Create: `discovery/hook-env-dump.sh`

**Step 1: Create a test hook script**

```bash
#!/bin/bash
# Dump all environment variables to a file for inspection
env | sort > /tmp/claude-hook-env-$(date +%s).txt
echo "Dumped env to /tmp/claude-hook-env-*.txt"
```

**Step 2: Temporarily add to Claude settings**

Manually add to `~/.claude/settings.json`:
```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "bash /Users/nishadsingh/repos/plate-spinner/discovery/hook-env-dump.sh"
          }
        ]
      }
    ]
  }
}
```

**Step 3: Run a Claude session and trigger a tool**

Start Claude, ask it to read a file. Check `/tmp/claude-hook-env-*.txt`.

**Step 4: Document findings**

Record in `discovery/FINDINGS.md`:
- Session ID variable name (if any)
- Project path variable
- Tool name / parameters variables
- TMUX_PANE availability

**Step 5: Clean up**

Remove the test hook from `~/.claude/settings.json`.

---

### Task 2: Discover Stop Hook Behavior

**Goal:** Confirm Stop hook exists and how to detect success vs error.

**Files:**
- Modify: `discovery/hook-env-dump.sh`

**Step 1: Add Stop hook to settings**

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "bash /Users/nishadsingh/repos/plate-spinner/discovery/hook-env-dump.sh stop"
          }
        ]
      }
    ]
  }
}
```

**Step 2: Modify dump script to capture hook type**

```bash
#!/bin/bash
HOOK_TYPE="${1:-unknown}"
env | sort > /tmp/claude-hook-env-${HOOK_TYPE}-$(date +%s).txt
```

**Step 3: Test success case**

Run Claude, complete a task successfully. Check dump.

**Step 4: Test error case**

Run Claude, cause an error. Check dump for differences.

**Step 5: Document findings**

Update `discovery/FINDINGS.md`:
- Does Stop hook fire?
- What env vars distinguish success/error?
- Any exit code or status variable?

---

### Task 3: Discover TodoWrite Parameters

**Goal:** Confirm hooks receive tool parameters for TodoWrite parsing.

**Files:**
- Modify: `discovery/hook-env-dump.sh`

**Step 1: Check existing dumps**

Look for `CLAUDE_TOOL_*` or similar variables containing parameters.

**Step 2: If not in env, check stdin**

Modify hook to capture stdin:
```bash
#!/bin/bash
HOOK_TYPE="${1:-unknown}"
TIMESTAMP=$(date +%s)
env | sort > /tmp/claude-hook-env-${HOOK_TYPE}-${TIMESTAMP}.txt
cat > /tmp/claude-hook-stdin-${HOOK_TYPE}-${TIMESTAMP}.txt
```

**Step 3: Trigger TodoWrite**

Ask Claude to create a todo list. Check dumps.

**Step 4: Document findings**

Update `discovery/FINDINGS.md`:
- How are tool parameters passed (env, stdin, args)?
- Format of TodoWrite payload

---

### Task 4: Set Up Python Project

**Goal:** Initialize Python project with uv and dependencies.

**Files:**
- Create: `pyproject.toml`
- Create: `src/plate_spinner/__init__.py`
- Create: `tests/__init__.py`

**Step 1: Initialize with uv**

Run: `uv init --lib --name plate-spinner`

**Step 2: Update pyproject.toml**

```toml
[project]
name = "plate-spinner"
version = "0.1.0"
description = "Dashboard for managing multiple Claude Code sessions"
readme = "README.md"
requires-python = ">=3.11"
dependencies = [
    "fastapi>=0.115.0",
    "uvicorn[standard]>=0.34.0",
    "textual>=0.89.0",
    "httpx>=0.28.0",
    "websockets>=14.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=8.0.0",
    "pytest-asyncio>=0.24.0",
]

[project.scripts]
sp = "plate_spinner.cli:main"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
```

**Step 3: Create package structure**

```
src/plate_spinner/
├── __init__.py
├── cli.py
├── daemon/
│   ├── __init__.py
│   ├── app.py
│   ├── db.py
│   └── models.py
└── tui/
    ├── __init__.py
    └── app.py
```

**Step 4: Install dependencies**

Run: `uv sync`

**Step 5: Verify setup**

Run: `uv run python -c "import plate_spinner; print('OK')"`
Expected: OK

**Step 6: Commit**

```bash
git add pyproject.toml src/ tests/
git commit -m "Initialize Python project structure"
```

---

### Task 5: Create SQLite Schema

**Goal:** Implement database schema and basic operations.

**Files:**
- Create: `src/plate_spinner/daemon/db.py`
- Create: `tests/test_db.py`

**Step 1: Write failing test for database creation**

```python
# tests/test_db.py
import tempfile
from pathlib import Path

import pytest

from plate_spinner.daemon.db import Database


def test_database_creates_tables():
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        db = Database(db_path)

        tables = db.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall()
        table_names = {t[0] for t in tables}

        assert "sessions" in table_names
        assert "todos" in table_names
        assert "events" in table_names
```

**Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_db.py -v`
Expected: FAIL (ModuleNotFoundError)

**Step 3: Implement Database class**

```python
# src/plate_spinner/daemon/db.py
import sqlite3
from pathlib import Path

SCHEMA = """
CREATE TABLE IF NOT EXISTS sessions (
    session_id TEXT PRIMARY KEY,
    project_path TEXT NOT NULL,
    tmux_pane TEXT,
    status TEXT NOT NULL DEFAULT 'running',
    last_event_type TEXT,
    last_tool TEXT,
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
"""


class Database:
    def __init__(self, db_path: Path):
        self.db_path = db_path
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        self.conn = sqlite3.connect(str(db_path), check_same_thread=False)
        self.conn.row_factory = sqlite3.Row
        self._init_schema()

    def _init_schema(self):
        self.conn.executescript(SCHEMA)
        self.conn.commit()

    def execute(self, sql: str, params: tuple = ()) -> sqlite3.Cursor:
        return self.conn.execute(sql, params)

    def commit(self):
        self.conn.commit()

    def close(self):
        self.conn.close()
```

**Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_db.py -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/plate_spinner/daemon/db.py tests/test_db.py
git commit -m "Add SQLite database with schema"
```

---

### Task 6: Create Data Models

**Goal:** Define Pydantic models for events and sessions.

**Files:**
- Create: `src/plate_spinner/daemon/models.py`
- Create: `tests/test_models.py`

**Step 1: Write failing test**

```python
# tests/test_models.py
from plate_spinner.daemon.models import HookEvent, Session, SessionStatus


def test_hook_event_from_dict():
    data = {
        "session_id": "abc123",
        "project_path": "/path/to/project",
        "event_type": "tool_call",
        "tool_name": "AskUserQuestion",
        "tmux_pane": "%5",
    }
    event = HookEvent(**data)
    assert event.session_id == "abc123"
    assert event.tool_name == "AskUserQuestion"


def test_session_status_from_tool():
    assert SessionStatus.from_tool("AskUserQuestion") == SessionStatus.AWAITING_INPUT
    assert SessionStatus.from_tool("ExitPlanMode") == SessionStatus.AWAITING_APPROVAL
    assert SessionStatus.from_tool("Read") == SessionStatus.RUNNING
```

**Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_models.py -v`
Expected: FAIL

**Step 3: Implement models**

```python
# src/plate_spinner/daemon/models.py
from datetime import datetime, timezone
from enum import Enum

from pydantic import BaseModel, Field


class SessionStatus(str, Enum):
    RUNNING = "running"
    IDLE = "idle"
    AWAITING_INPUT = "awaiting_input"
    AWAITING_APPROVAL = "awaiting_approval"
    ERROR = "error"

    @classmethod
    def from_tool(cls, tool_name: str) -> "SessionStatus":
        if tool_name == "AskUserQuestion":
            return cls.AWAITING_INPUT
        if tool_name == "ExitPlanMode":
            return cls.AWAITING_APPROVAL
        return cls.RUNNING


class HookEvent(BaseModel):
    session_id: str
    project_path: str
    event_type: str
    tool_name: str | None = None
    tool_params: dict | None = None
    tmux_pane: str | None = None
    error: str | None = None
    timestamp: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))


class Session(BaseModel):
    session_id: str
    project_path: str
    tmux_pane: str | None = None
    status: SessionStatus = SessionStatus.RUNNING
    last_event_type: str | None = None
    last_tool: str | None = None
    created_at: datetime
    updated_at: datetime

    @property
    def project_name(self) -> str:
        return self.project_path.rstrip("/").split("/")[-1]

    @property
    def needs_attention(self) -> bool:
        return self.status in (
            SessionStatus.AWAITING_INPUT,
            SessionStatus.AWAITING_APPROVAL,
            SessionStatus.IDLE,
            SessionStatus.ERROR,
        )
```

**Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_models.py -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/plate_spinner/daemon/models.py tests/test_models.py
git commit -m "Add Pydantic models for events and sessions"
```

---

## Phase 2: Hooks → Daemon

### Task 7: Create FastAPI App with Events Endpoint

**Goal:** Implement POST /events endpoint.

**Files:**
- Create: `src/plate_spinner/daemon/app.py`
- Create: `tests/test_app.py`

**Step 1: Write failing test**

```python
# tests/test_app.py
import tempfile
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

from plate_spinner.daemon.app import create_app
from plate_spinner.daemon.db import Database


@pytest.fixture
def client():
    with tempfile.TemporaryDirectory() as tmpdir:
        db = Database(Path(tmpdir) / "test.db")
        app = create_app(db)
        yield TestClient(app)


def test_post_event_creates_session(client):
    response = client.post("/events", json={
        "session_id": "abc123",
        "project_path": "/path/to/project",
        "event_type": "tool_call",
        "tool_name": "Read",
    })
    assert response.status_code == 200

    sessions = client.get("/sessions").json()
    assert len(sessions) == 1
    assert sessions[0]["session_id"] == "abc123"
    assert sessions[0]["status"] == "running"


def test_ask_user_question_sets_awaiting_input(client):
    client.post("/events", json={
        "session_id": "abc123",
        "project_path": "/path/to/project",
        "event_type": "tool_call",
        "tool_name": "AskUserQuestion",
    })

    sessions = client.get("/sessions").json()
    assert sessions[0]["status"] == "awaiting_input"
```

**Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_app.py -v`
Expected: FAIL

**Step 3: Implement FastAPI app**

```python
# src/plate_spinner/daemon/app.py
from datetime import datetime, timezone

from fastapi import FastAPI

from .db import Database
from .models import HookEvent, Session, SessionStatus


def create_app(db: Database) -> FastAPI:
    app = FastAPI(title="Plate-Spinner Daemon")

    @app.post("/events")
    async def post_event(event: HookEvent):
        now = datetime.now(timezone.utc).isoformat()

        existing = db.execute(
            "SELECT session_id FROM sessions WHERE session_id = ?",
            (event.session_id,)
        ).fetchone()

        if event.event_type == "stop":
            status = SessionStatus.ERROR if event.error else SessionStatus.IDLE
        else:
            status = SessionStatus.from_tool(event.tool_name or "")

        if existing:
            db.execute(
                """UPDATE sessions SET
                   status = ?, last_event_type = ?, last_tool = ?,
                   tmux_pane = COALESCE(?, tmux_pane), updated_at = ?
                   WHERE session_id = ?""",
                (status.value, event.event_type, event.tool_name,
                 event.tmux_pane, now, event.session_id)
            )
        else:
            db.execute(
                """INSERT INTO sessions
                   (session_id, project_path, tmux_pane, status,
                    last_event_type, last_tool, created_at, updated_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?)""",
                (event.session_id, event.project_path, event.tmux_pane,
                 status.value, event.event_type, event.tool_name, now, now)
            )

        db.execute(
            """INSERT INTO events (session_id, event_type, payload, created_at)
               VALUES (?, ?, ?, ?)""",
            (event.session_id, event.event_type, event.model_dump_json(), now)
        )
        db.commit()

        return {"status": "ok"}

    @app.get("/sessions")
    async def get_sessions() -> list[dict]:
        rows = db.execute(
            """SELECT session_id, project_path, tmux_pane, status,
                      last_event_type, last_tool, created_at, updated_at
               FROM sessions ORDER BY updated_at DESC"""
        ).fetchall()
        return [dict(row) for row in rows]

    return app
```

**Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_app.py -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/plate_spinner/daemon/app.py tests/test_app.py
git commit -m "Add FastAPI app with events endpoint"
```

---

### Task 8: Create Plugin Structure

**Goal:** Create Claude Code plugin with hook scripts.

**Files:**
- Create: `plugin/plugin.json`
- Create: `plugin/hooks/hooks.json`
- Create: `plugin/scripts/post-tool-use.sh`
- Create: `plugin/scripts/stop.sh`

**Step 1: Create plugin.json**

```json
{
  "name": "plate-spinner-hooks",
  "version": "0.1.0",
  "description": "Hooks for Plate-Spinner session tracking"
}
```

**Step 2: Create hooks.json**

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "[ \"$PLATE_SPINNER\" = \"1\" ] && \"$CLAUDE_PROJECT_DIR/../plate-spinner/plugin/scripts/post-tool-use.sh\" || true"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "[ \"$PLATE_SPINNER\" = \"1\" ] && \"$CLAUDE_PROJECT_DIR/../plate-spinner/plugin/scripts/stop.sh\" || true"
          }
        ]
      }
    ]
  }
}
```

Note: Hook paths will be updated after discovery tasks reveal actual env vars.

**Step 3: Create post-tool-use.sh**

```bash
#!/bin/bash
set -e

# Skip if daemon not running
curl -s --connect-timeout 1 http://localhost:7890/health >/dev/null 2>&1 || exit 0

# Build event payload (update vars after discovery)
SESSION_ID="${CLAUDE_SESSION_ID:-unknown}"
PROJECT_PATH="${CLAUDE_PROJECT_DIR:-$(pwd)}"
TOOL_NAME="${CLAUDE_TOOL_NAME:-unknown}"

curl -s -X POST http://localhost:7890/events \
  -H "Content-Type: application/json" \
  -d "{
    \"session_id\": \"$SESSION_ID\",
    \"project_path\": \"$PROJECT_PATH\",
    \"event_type\": \"tool_call\",
    \"tool_name\": \"$TOOL_NAME\",
    \"tmux_pane\": \"$TMUX_PANE\"
  }" >/dev/null 2>&1 || true
```

**Step 4: Create stop.sh**

```bash
#!/bin/bash
set -e

curl -s --connect-timeout 1 http://localhost:7890/health >/dev/null 2>&1 || exit 0

SESSION_ID="${CLAUDE_SESSION_ID:-unknown}"
PROJECT_PATH="${CLAUDE_PROJECT_DIR:-$(pwd)}"
ERROR="${CLAUDE_ERROR:-}"

EVENT_TYPE="stop"
ERROR_JSON="null"
if [ -n "$ERROR" ]; then
  ERROR_JSON="\"$ERROR\""
fi

curl -s -X POST http://localhost:7890/events \
  -H "Content-Type: application/json" \
  -d "{
    \"session_id\": \"$SESSION_ID\",
    \"project_path\": \"$PROJECT_PATH\",
    \"event_type\": \"$EVENT_TYPE\",
    \"error\": $ERROR_JSON,
    \"tmux_pane\": \"$TMUX_PANE\"
  }" >/dev/null 2>&1 || true
```

**Step 5: Make scripts executable**

Run: `chmod +x plugin/scripts/*.sh`

**Step 6: Commit**

```bash
git add plugin/
git commit -m "Add Claude Code plugin with hooks"
```

---

### Task 9: Add Health Endpoint and WebSocket

**Goal:** Add /health endpoint and WebSocket for TUI.

**Files:**
- Modify: `src/plate_spinner/daemon/app.py`
- Modify: `tests/test_app.py`

**Step 1: Write failing test for health endpoint**

```python
# Add to tests/test_app.py
def test_health_endpoint(client):
    response = client.get("/health")
    assert response.status_code == 200
    assert response.json() == {"status": "ok"}
```

**Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_app.py::test_health_endpoint -v`
Expected: FAIL

**Step 3: Add health endpoint and WebSocket setup**

```python
# Update src/plate_spinner/daemon/app.py
from datetime import datetime, timezone
import asyncio
import json

from fastapi import FastAPI, WebSocket, WebSocketDisconnect

from .db import Database
from .models import HookEvent, Session, SessionStatus


class ConnectionManager:
    def __init__(self):
        self.connections: list[WebSocket] = []

    async def connect(self, websocket: WebSocket):
        await websocket.accept()
        self.connections.append(websocket)

    def disconnect(self, websocket: WebSocket):
        self.connections.remove(websocket)

    async def broadcast(self, message: dict):
        for connection in self.connections:
            try:
                await connection.send_json(message)
            except Exception:
                pass


def create_app(db: Database) -> FastAPI:
    app = FastAPI(title="Plate-Spinner Daemon")
    manager = ConnectionManager()

    @app.get("/health")
    async def health():
        return {"status": "ok"}

    @app.post("/events")
    async def post_event(event: HookEvent):
        now = datetime.now(timezone.utc).isoformat()

        existing = db.execute(
            "SELECT session_id FROM sessions WHERE session_id = ?",
            (event.session_id,)
        ).fetchone()

        if event.event_type == "stop":
            status = SessionStatus.ERROR if event.error else SessionStatus.IDLE
        else:
            status = SessionStatus.from_tool(event.tool_name or "")

        if existing:
            db.execute(
                """UPDATE sessions SET
                   status = ?, last_event_type = ?, last_tool = ?,
                   tmux_pane = COALESCE(?, tmux_pane), updated_at = ?
                   WHERE session_id = ?""",
                (status.value, event.event_type, event.tool_name,
                 event.tmux_pane, now, event.session_id)
            )
        else:
            db.execute(
                """INSERT INTO sessions
                   (session_id, project_path, tmux_pane, status,
                    last_event_type, last_tool, created_at, updated_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?)""",
                (event.session_id, event.project_path, event.tmux_pane,
                 status.value, event.event_type, event.tool_name, now, now)
            )

        db.execute(
            """INSERT INTO events (session_id, event_type, payload, created_at)
               VALUES (?, ?, ?, ?)""",
            (event.session_id, event.event_type, event.model_dump_json(), now)
        )
        db.commit()

        # Broadcast update to TUI
        await manager.broadcast({"type": "session_update", "session_id": event.session_id})

        return {"status": "ok"}

    @app.get("/sessions")
    async def get_sessions() -> list[dict]:
        rows = db.execute(
            """SELECT session_id, project_path, tmux_pane, status,
                      last_event_type, last_tool, created_at, updated_at
               FROM sessions ORDER BY updated_at DESC"""
        ).fetchall()
        return [dict(row) for row in rows]

    @app.websocket("/ws")
    async def websocket_endpoint(websocket: WebSocket):
        await manager.connect(websocket)
        try:
            while True:
                await websocket.receive_text()
        except WebSocketDisconnect:
            manager.disconnect(websocket)

    return app
```

**Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_app.py -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/plate_spinner/daemon/app.py tests/test_app.py
git commit -m "Add health endpoint and WebSocket support"
```

---

## Phase 3: TUI

### Task 10: Create Basic TUI Layout

**Goal:** Build Textual TUI with session list.

**Files:**
- Create: `src/plate_spinner/tui/app.py`

**Step 1: Implement TUI app**

```python
# src/plate_spinner/tui/app.py
from textual.app import App, ComposeResult
from textual.containers import Container, Vertical, VerticalScroll
from textual.widgets import Footer, Header, Static
from textual.binding import Binding

import httpx


class SessionWidget(Static):
    def __init__(self, session: dict, index: int):
        self.session = session
        self.index = index
        super().__init__()

    def compose(self) -> ComposeResult:
        status = self.session["status"]
        project = self.session["project_path"].rstrip("/").split("/")[-1]

        status_icons = {
            "running": ">",
            "idle": "-",
            "awaiting_input": "?",
            "awaiting_approval": "!",
            "error": "X",
        }
        icon = status_icons.get(status, " ")

        yield Static(f"[{self.index}] {icon} {project:<20} {status}")


class SessionGroup(Static):
    def __init__(self, title: str, sessions: list[dict], start_index: int):
        self.title = title
        self.sessions = sessions
        self.start_index = start_index
        super().__init__()

    def compose(self) -> ComposeResult:
        yield Static(f"\n{self.title}", classes="group-title")
        for i, session in enumerate(self.sessions):
            yield SessionWidget(session, self.start_index + i + 1)


class PlateSpinnerApp(App):
    CSS = """
    .group-title {
        color: $text-muted;
        text-style: bold;
    }
    SessionWidget {
        padding: 0 1;
    }
    """

    BINDINGS = [
        Binding("q", "quit", "Quit"),
        Binding("r", "refresh", "Refresh"),
        Binding("d", "dismiss", "Dismiss"),
        Binding("1", "jump(1)", "Jump 1", show=False),
        Binding("2", "jump(2)", "Jump 2", show=False),
        Binding("3", "jump(3)", "Jump 3", show=False),
        Binding("4", "jump(4)", "Jump 4", show=False),
        Binding("5", "jump(5)", "Jump 5", show=False),
        Binding("6", "jump(6)", "Jump 6", show=False),
        Binding("7", "jump(7)", "Jump 7", show=False),
        Binding("8", "jump(8)", "Jump 8", show=False),
        Binding("9", "jump(9)", "Jump 9", show=False),
    ]

    def __init__(self, daemon_url: str = "http://localhost:7890"):
        super().__init__()
        self.daemon_url = daemon_url
        self.sessions: list[dict] = []

    def compose(self) -> ComposeResult:
        yield Header()
        yield VerticalScroll(id="main")
        yield Footer()

    async def on_mount(self):
        self.title = "Plate-Spinner"
        await self.action_refresh()

    async def action_refresh(self):
        try:
            async with httpx.AsyncClient() as client:
                response = await client.get(f"{self.daemon_url}/sessions")
                self.sessions = response.json()
        except Exception:
            self.sessions = []

        self.render_sessions()

    def render_sessions(self):
        main = self.query_one("#main")
        main.remove_children()

        needs_attention = [s for s in self.sessions if s["status"] in
                          ("awaiting_input", "awaiting_approval", "error")]
        running = [s for s in self.sessions if s["status"] == "running"]
        idle = [s for s in self.sessions if s["status"] == "idle"]

        attention_count = len(needs_attention)
        self.sub_title = f"{attention_count} need attention" if attention_count else ""

        idx = 0
        if needs_attention:
            main.mount(SessionGroup("NEEDS ATTENTION", needs_attention, idx))
            idx += len(needs_attention)
        if running:
            main.mount(SessionGroup("RUNNING", running, idx))
            idx += len(running)
        if idle:
            main.mount(SessionGroup("IDLE", idle, idx))

    def action_jump(self, index: int):
        if index <= len(self.sessions):
            session = self.sessions[index - 1]
            pane = session.get("tmux_pane")
            if pane:
                import subprocess
                subprocess.run(["tmux", "select-pane", "-t", pane], check=False)

    def action_dismiss(self):
        pass  # TODO: implement session dismissal


def run():
    app = PlateSpinnerApp()
    app.run()
```

**Step 2: Test manually**

Run: `uv run python -c "from plate_spinner.tui.app import run; run()"`

**Step 3: Commit**

```bash
git add src/plate_spinner/tui/app.py
git commit -m "Add Textual TUI with session list"
```

---

### Task 11: Add WebSocket Real-time Updates

**Goal:** Connect TUI to daemon WebSocket for live updates.

**Files:**
- Modify: `src/plate_spinner/tui/app.py`

**Step 1: Add WebSocket connection**

```python
# Add to PlateSpinnerApp class
import asyncio
import websockets

async def connect_websocket(self):
    ws_url = self.daemon_url.replace("http://", "ws://") + "/ws"
    try:
        async with websockets.connect(ws_url) as ws:
            async for message in ws:
                await self.action_refresh()
    except Exception:
        # Retry connection after delay
        await asyncio.sleep(5)
        self.run_worker(self.connect_websocket())

async def on_mount(self):
    self.title = "Plate-Spinner"
    await self.action_refresh()
    self.run_worker(self.connect_websocket())
```

**Step 2: Test with running daemon**

Start daemon: `uv run python -c "..."`
Start TUI: `uv run python -c "from plate_spinner.tui.app import run; run()"`
Post event: `curl -X POST http://localhost:7890/events -H "Content-Type: application/json" -d '{"session_id":"test","project_path":"/tmp","event_type":"tool_call","tool_name":"Read"}'`

**Step 3: Commit**

```bash
git add src/plate_spinner/tui/app.py
git commit -m "Add WebSocket real-time updates to TUI"
```

---

## Phase 4: CLI & Polish

### Task 12: Create CLI Entry Point

**Goal:** Implement `sp` command with subcommands.

**Files:**
- Create: `src/plate_spinner/cli.py`

**Step 1: Implement CLI**

```python
# src/plate_spinner/cli.py
import argparse
import os
import subprocess
import sys
from pathlib import Path

import uvicorn

from .daemon.app import create_app
from .daemon.db import Database


def get_db_path() -> Path:
    return Path.home() / ".plate-spinner" / "state.db"


def daemon_running() -> bool:
    import httpx
    try:
        httpx.get("http://localhost:7890/health", timeout=1)
        return True
    except Exception:
        return False


def cmd_daemon(args):
    db = Database(get_db_path())
    app = create_app(db)
    uvicorn.run(app, host="127.0.0.1", port=7890, log_level="warning")


def cmd_tui(args):
    if not daemon_running():
        # Start daemon in background
        subprocess.Popen(
            [sys.executable, "-m", "plate_spinner.cli", "daemon"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
        import time
        time.sleep(1)

    from .tui.app import run
    run()


def cmd_run(args):
    env = os.environ.copy()
    env["PLATE_SPINNER"] = "1"
    os.execvpe("claude", ["claude"] + args.claude_args, env)


def cmd_sessions(args):
    import httpx
    import json
    try:
        response = httpx.get("http://localhost:7890/sessions", timeout=5)
        print(json.dumps(response.json(), indent=2))
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


def main():
    parser = argparse.ArgumentParser(prog="sp", description="Plate-Spinner")
    subparsers = parser.add_subparsers(dest="command")

    subparsers.add_parser("daemon", help="Run daemon in foreground")
    subparsers.add_parser("tui", help="Launch TUI")
    subparsers.add_parser("sessions", help="List sessions as JSON")

    run_parser = subparsers.add_parser("run", help="Launch Claude with tracking")
    run_parser.add_argument("claude_args", nargs="*", default=[])

    args = parser.parse_args()

    if args.command == "daemon":
        cmd_daemon(args)
    elif args.command == "run":
        cmd_run(args)
    elif args.command == "sessions":
        cmd_sessions(args)
    else:
        # Default: TUI
        cmd_tui(args)


if __name__ == "__main__":
    main()
```

**Step 2: Test CLI**

Run: `uv run sp --help`
Run: `uv run sp sessions`

**Step 3: Commit**

```bash
git add src/plate_spinner/cli.py
git commit -m "Add CLI with daemon, tui, run, sessions commands"
```

---

### Task 13: Add TodoWrite Parsing

**Goal:** Parse TodoWrite payloads for task progress display.

**Files:**
- Modify: `src/plate_spinner/daemon/app.py`
- Modify: `tests/test_app.py`

**Step 1: Write failing test**

```python
# Add to tests/test_app.py
def test_todowrite_stores_todos(client):
    client.post("/events", json={
        "session_id": "abc123",
        "project_path": "/path/to/project",
        "event_type": "tool_call",
        "tool_name": "TodoWrite",
        "tool_params": {
            "todos": [
                {"content": "Task 1", "status": "completed"},
                {"content": "Task 2", "status": "in_progress"},
                {"content": "Task 3", "status": "pending"},
            ]
        }
    })

    sessions = client.get("/sessions").json()
    assert sessions[0]["todo_progress"] == "2/3"
```

**Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_app.py::test_todowrite_stores_todos -v`
Expected: FAIL

**Step 3: Implement TodoWrite handling**

Update `/events` endpoint to store todos and compute progress.
Update `/sessions` to include `todo_progress`.

**Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_app.py -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/plate_spinner/daemon/app.py tests/test_app.py
git commit -m "Add TodoWrite parsing for task progress"
```

---

### Task 14: Update Hook Scripts After Discovery

**Goal:** Update hook scripts with correct env vars from discovery.

**Files:**
- Modify: `plugin/scripts/post-tool-use.sh`
- Modify: `plugin/scripts/stop.sh`

**Step 1: Review discovery findings**

Check `discovery/FINDINGS.md` for actual variable names.

**Step 2: Update scripts with correct vars**

Replace placeholder variable names with discovered ones.

**Step 3: Test end-to-end**

1. Start daemon: `sp daemon`
2. In new terminal: `sp run`
3. Trigger some tool calls
4. Check: `sp sessions`

**Step 4: Commit**

```bash
git add plugin/scripts/
git commit -m "Update hooks with discovered env vars"
```

---

### Task 15: Final Polish and README

**Goal:** Add README and verify install flow.

**Files:**
- Create: `README.md`

**Step 1: Write README**

```markdown
# Plate-Spinner

Dashboard for managing multiple Claude Code sessions.

## Install

```bash
pip install plate-spinner
claude plugin add plate-spinner-hooks
```

## Usage

```bash
# Launch a tracked Claude session
sp run

# Open dashboard
sp

# List sessions (JSON)
sp sessions
```

## Requirements

- Python 3.11+
- Claude Code
- tmux (for jump-to-session)
```

**Step 2: Test full install flow**

1. `pip install -e .`
2. Add plugin manually (or test plugin install)
3. `sp run` → `sp`

**Step 3: Commit**

```bash
git add README.md
git commit -m "Add README"
```

---

## Summary

| Phase | Tasks | What's Built |
|-------|-------|--------------|
| 1 | 1-6 | Discovery, Python project, DB, models |
| 2 | 7-9 | FastAPI daemon, plugin hooks, WebSocket |
| 3 | 10-11 | Textual TUI with real-time updates |
| 4 | 12-15 | CLI, TodoWrite parsing, polish |
