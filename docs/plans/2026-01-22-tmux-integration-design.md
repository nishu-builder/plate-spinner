# tmux Integration Design

## Overview

Wrap `sp run` sessions in tmux, enabling programmatic prompt injection and session resume.

## Behavior

`sp run [args...]` will:
1. Check tmux is installed (require 3.2+ for `-e` flag)
2. Generate session name: `sp-{random7chars}`
3. Create tmux session/window and run claude inside it
4. When claude exits, tmux cleans up automatically

### Already in tmux?

- **No** - `tmux new-session -s sp-xxx ...`
- **Yes** - `tmux new-window -n sp-xxx ...` (detected via `$TMUX` env var)

### tmux commands

```bash
# Not in tmux
tmux new-session -s sp-abc1234 \
  -e PLATE_SPINNER=1 \
  -e PLATE_SPINNER_TMUX_TARGET=sp-abc1234 \
  -- claude [args]

# Already in tmux
tmux new-window -n sp-abc1234 \
  -e PLATE_SPINNER=1 \
  -e PLATE_SPINNER_TMUX_TARGET=sp-abc1234 \
  -- claude [args]
```

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
3. Check `$TMUX` env var for nested detection
4. Generate random 7-char session name
5. Exec appropriate tmux command

Signal handling removed - tmux manages the process lifecycle.

`notify_stopped` still needed - call after tmux command returns.

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
