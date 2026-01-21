# Plate-Spinner

Dashboard for managing multiple concurrent Claude Code sessions.

## Problem

When running multiple Claude Code sessions, you lose track of which need input, which are blocked, and which are working. Plate-Spinner provides unified visibility with a terminal UI.

## Install

```bash
pip install plate-spinner
sp install
```

Then add the hooks config printed by `sp install` to `~/.claude/settings.json`.

## Usage

```bash
# Launch a tracked Claude session
sp run

# Open dashboard
sp

# Other commands
sp daemon     # Run daemon in foreground
sp sessions   # List sessions as JSON
```

## How It Works

```
Claude Code Session
       |
       | hooks fire on tool calls
       v
~/.plate-spinner/hooks/*.sh
       |
       | POST to localhost:7890
       v
   sp daemon (SQLite + WebSocket)
       |
       +---> sp tui (real-time updates)
```

## Requirements

- Python 3.11+
- Claude Code
- tmux (for jump-to-session)
- jq (recommended, for JSON transformation)

## Development

```bash
git clone https://github.com/yourusername/plate-spinner
cd plate-spinner
uv sync
uv run pytest
```
