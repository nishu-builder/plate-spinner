# Plate-Spinner Design

A dashboard for managing multiple concurrent Claude Code sessions.

## Problem

When running multiple Claude Code sessions, you lose track of which sessions need your input, which are blocked, and which are working. You end up context-switching between terminals and missing when agents need you.

## Solution

Plate-Spinner provides unified visibility into all active Claude Code sessions with a terminal UI showing status, plus quick jump to the right tmux pane.

## Architecture

```
Claude Code Session
       │
       │ hooks fire on tool calls
       ▼
Claude Code Plugin (plate-spinner-hooks)
       │
       │ POST to localhost:7890
       ▼
   sp daemon (SQLite + WebSocket)
       │
       └──► sp tui
```

No Superpowers fork. Works with any Claude Code session.

## Components

### 1. Claude Code Plugin (`plate-spinner-hooks`)

Installed via `claude plugin add plate-spinner-hooks`.

Contains hooks for `PostToolUse` and `Stop` events that POST to the daemon. Hooks are guarded by `PLATE_SPINNER=1` env var so they don't affect normal Claude usage.

```
plate-spinner-hooks/
├── plugin.json
├── hooks/
│   └── hooks.json
└── scripts/
    ├── post-tool-use.sh
    └── stop.sh
```

### 2. Daemon (`sp daemon`)

Python FastAPI server on `localhost:7890`.

**Endpoints:**
- `POST /events` — receive events from hooks
- `GET /sessions` — list all sessions
- `GET /ws` — WebSocket for TUI

**SQLite schema:**

```sql
CREATE TABLE sessions (
  session_id TEXT PRIMARY KEY,
  project_path TEXT NOT NULL,
  tmux_pane TEXT,
  status TEXT NOT NULL,  -- running, idle, awaiting_input, awaiting_approval, error
  last_event_type TEXT,
  last_tool TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE todos (
  session_id TEXT PRIMARY KEY REFERENCES sessions(session_id),
  todos_json TEXT,
  updated_at TEXT NOT NULL
);

CREATE TABLE events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  payload TEXT NOT NULL,
  created_at TEXT NOT NULL
);
```

**State machine:**
```
running ──► idle (Stop hook)
running ──► awaiting_input (AskUserQuestion)
running ──► awaiting_approval (ExitPlanMode)
running ──► error (Stop hook with error)
* ──► running (any tool call)
```

### 3. TUI (`sp`)

Textual app showing all sessions grouped by status.

```
┌─ Plate-Spinner ──────────────────────────── 2 need attention ─┐
│                                                                │
│ NEEDS ATTENTION                                                │
│ ┌────────────────────────────────────────────────────────────┐ │
│ │ [1] plate-spinner        awaiting_input                    │ │
│ │     "Which tech stack?"                                    │ │
│ ├────────────────────────────────────────────────────────────┤ │
│ │ [2] wedding-vows         awaiting_approval                 │ │
│ │     Plan ready for review                                  │ │
│ └────────────────────────────────────────────────────────────┘ │
│                                                                │
│ RUNNING                                                        │
│ ┌────────────────────────────────────────────────────────────┐ │
│ │ [3] api-refactor         running                           │ │
│ │     Task 3/7: Update auth middleware                       │ │
│ └────────────────────────────────────────────────────────────┘ │
│                                                                │
│ IDLE                                                           │
│ ┌────────────────────────────────────────────────────────────┐ │
│ │ [4] docs-site            idle                              │ │
│ └────────────────────────────────────────────────────────────┘ │
│                                                                │
├────────────────────────────────────────────────────────────────┤
│ [1-9] Jump  [d] Dismiss  [q] Quit                              │
└────────────────────────────────────────────────────────────────┘
```

**Features:**
- Real-time updates via WebSocket
- Number keys jump to tmux pane
- Sessions grouped by status, sorted by last activity
- Shows todo progress from TodoWrite parsing

### 4. CLI

```bash
sp              # Launch TUI (starts daemon if not running)
sp run          # Launch Claude with PLATE_SPINNER=1
sp daemon       # Run daemon in foreground
sp sessions     # List sessions as JSON
```

## "Needs Attention" States

| State | Detection |
|-------|-----------|
| `awaiting_input` | `AskUserQuestion` tool call |
| `awaiting_approval` | `ExitPlanMode` tool call |
| `idle` | Stop hook fires (success) |
| `error` | Stop hook fires (with error) |
| `running` | Tool calls happening |

## Tech Stack

- **Daemon**: Python + FastAPI + SQLite
- **TUI**: Python + Textual
- **Hooks**: Bash + curl
- **Distribution**: pip package + Claude Code plugin

## Installation

```bash
pip install plate-spinner
claude plugin add plate-spinner-hooks
```

## Usage

```bash
sp run              # launch tracked Claude session
sp                  # open dashboard
```

## Data Location

```
~/.plate-spinner/
├── state.db
└── logs/
```

## Discovery Tasks

Before implementation, need to discover:
1. What env vars are available in hooks (session ID, project path)
2. Stop hook behavior (success vs error differentiation)
3. Hook parameter access (can we see TodoWrite payload?)

## Implementation Phases

**Phase 1: Discovery & Foundation**
1. Discover hook env vars and behavior
2. Set up Python project structure
3. Create SQLite schema and basic daemon

**Phase 2: Hooks → Daemon**
4. Create plugin structure with hook scripts
5. Implement `POST /events` endpoint
6. Wire up hooks to POST events

**Phase 3: TUI**
7. Build Textual TUI with session list
8. Add WebSocket for real-time updates
9. Implement tmux jump-to-session

**Phase 4: Polish**
10. `sp run` wrapper command
11. TodoWrite parsing for task progress
12. Error handling and docs

## Not in v1

- Push notifications (ntfy, etc.)
- Inline responding to questions
- History/analytics view
- Multi-machine sync
- Web dashboard
