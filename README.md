# Plate-Spinner

Dashboard for managing multiple concurrent Claude Code sessions.

## Installation

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/nishu-builder/plate-spinner/releases/latest/download/plate-spinner-installer.sh | sh
```

Or build from source:

```bash
cargo install --git https://github.com/nishu-builder/plate-spinner
```

## Quick Start

```bash
sp install              # Prints hook config to add to ~/.claude/settings.json
sp auth set             # Configure API key (optional, enables summaries)
sp                      # Open dashboard (terminal 1)
sp run                  # Start tracked plate (terminal 2)
sp run                  # Start another (terminal 3)
```

<img src="assets/screenshot.png" alt="Plate-Spinner screenshot" width="900"/>



## Plate States

| Icon | Status | Trigger |
|------|--------|---------|
| `.` | starting | Plate registered, no activity yet |
| `>` | running | Tool executing |
| `?` | awaiting_input | `AskUserQuestion` called |
| `!` | awaiting_approval | `ExitPlanMode` called |
| `-` | idle | Stop event received |
| `X` | error | Stop event with error |
| `x` | closed | Plate wrapper exited |

AI summaries appear when plates reach a waiting state (requires API key, see Authentication below).

## Commands

```
sp              Dashboard (auto-starts daemon)
sp run [args]   Launch Claude with tracking
sp install      Print settings.json hook config
sp kill         Stop daemon
sp plates       List plates as JSON
sp daemon       Run daemon in foreground
sp auth         Show authentication status
  set           Set API key (prompted)
  unset         Remove stored API key
  path          Print auth config path
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

## Requirements

- Claude Code

## Authentication

AI summaries require an Anthropic API key. Configure it with:

```bash
sp auth set
```

The key is stored in `~/.config/plate-spinner/auth.toml` with restricted permissions (0600).

Alternatively, set the `ANTHROPIC_API_KEY` environment variable (takes precedence over the stored key).

## Development

```bash
cargo build
sp              # daemon auto-restarts if binary changed
```

The daemon includes a build timestamp, so any `sp` command after rebuilding will detect the version mismatch and restart the daemon automatically. TUI changes require quitting (`q`) and restarting `sp`.

### Testing and Linting

```bash
cargo test                                            # run tests
cargo fmt --all -- --check                            # check formatting
cargo clippy --all-targets --all-features -- -D warnings  # run linter
```

### Git Hooks

Install pre-commit hooks to run formatting and linting checks before each commit:

```bash
./scripts/install-hooks.sh
```
