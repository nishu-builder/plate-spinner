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
