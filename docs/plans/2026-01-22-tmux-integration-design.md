# tmux Integration Design

## Overview

Wrap `sp run` sessions in tmux, enabling programmatic prompt injection and session resume.

## Behavior

Both `sp` (TUI) and `sp run` (Claude instances) operate in tmux:

### Session strategy

- **Not in tmux** - Use/create session named `plate-spinner`
- **Already in tmux** - Use current session (detected via `$TMUX` env var)

All windows live in the same session for easy switching (`ctrl-b 0/1/2`).

### `sp run [args...]`

1. Check tmux is installed (require 3.2+ for `-e` flag)
2. Generate window name: `sp-{random7chars}`
3. Create window in appropriate session, run claude inside it
4. When claude exits, window auto-closes

### `sp` (TUI)

1. Create window named `dashboard` in appropriate session
2. Run TUI in it
3. When TUI exits, window auto-closes

### tmux commands

**For `sp run`:**

```bash
# Not in tmux - create/join "plate-spinner" session
if ! tmux has-session -t plate-spinner 2>/dev/null; then
  tmux new-session -d -s plate-spinner
fi
tmux new-window -t plate-spinner: -n sp-abc1234 \
  -e PLATE_SPINNER=1 \
  -e PLATE_SPINNER_TMUX_TARGET=plate-spinner:sp-abc1234 \
  -- claude [args]
tmux attach -t plate-spinner:sp-abc1234

# Already in tmux - use current session
tmux new-window -n sp-abc1234 \
  -e PLATE_SPINNER=1 \
  -e PLATE_SPINNER_TMUX_TARGET=$TMUX_SESSION:sp-abc1234 \
  -- claude [args]
```

**For `sp` (TUI):**

```bash
# Same logic, but window name is "dashboard"
if ! tmux has-session -t plate-spinner 2>/dev/null; then
  tmux new-session -d -s plate-spinner
fi
tmux new-window -t plate-spinner: -n dashboard -- sp-tui
tmux attach -t plate-spinner:dashboard
```

**tmux_target format:** `{session}:{window}` enables `tmux send-keys -t {target}`

### Error: tmux not installed

```
Error: tmux is required for sp run. Install it with:
  brew install tmux    # macOS
  apt install tmux     # Debian/Ubuntu
```

## Database Changes

Add `tmux_target` column to plates table:

```sql
ALTER TABLE plates ADD COLUMN tmux_target TEXT;
```

Migration approach: idempotent check on startup. If column doesn't exist, add it.

## Implementation

### src/cli/run.rs

Replace fork/exec with:
1. `which tmux` - check installed
2. `tmux -V` - check version >= 3.2
3. Check `$TMUX` env var to detect if in tmux
4. Generate random 7-char window name
5. Determine session name:
   - If `$TMUX` set: extract current session name
   - Else: use `plate-spinner`
6. If not in tmux and session doesn't exist: create it detached
7. Create window with `tmux new-window`
8. Attach to the window
9. After tmux returns: call `notify_stopped`

Signal handling removed - tmux manages the process lifecycle.

### src/cli/mod.rs (main CLI entry)

When user runs `sp` (no subcommand), wrap TUI launch in tmux:
1. Same session detection logic as `sp run`
2. Create window named `dashboard`
3. Run the TUI inside it

### src/hook/session_start.rs

Read `PLATE_SPINNER_TMUX_TARGET` from environment, include in event payload.

### src/db.rs

- Add `ensure_tmux_target_column()` migration function
- Call from `init_db()`
- Update `upsert_plate()` to accept and store `tmux_target`

### src/daemon/handlers.rs

Parse `tmux_target` from session_start event payload, pass to database layer.

## Future Use

Once stored, `tmux_target` enables:
- `tmux send-keys -t sp-abc1234 "prompt text" Enter` - inject prompts
- `tmux attach -t sp-abc1234` - resume detached session
- Event inbox dispatch to sessions
