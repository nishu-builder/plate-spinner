# Claude Code Hooks Discovery Findings

## Key Finding

**Hook data is passed via stdin as JSON**, not environment variables.

## PostToolUse Hook

**Stdin JSON structure:**
```json
{
  "session_id": "34dd35d1-709d-4d9a-890e-c313e8ae5417",
  "transcript_path": "/Users/nishadsingh/.claude/projects/-private-tmp/34dd35d1-709d-4d9a-890e-c313e8ae5417.jsonl",
  "cwd": "/private/tmp",
  "permission_mode": "default",
  "hook_event_name": "PostToolUse",
  "tool_name": "TodoWrite",
  "tool_input": { ... },
  "tool_response": { ... },
  "tool_use_id": "toolu_..."
}
```

**Available fields:**
- `session_id` - UUID identifying the session (persists across resume)
- `transcript_path` - Path to session transcript
- `cwd` - Current working directory (project path)
- `tool_name` - Name of tool called (e.g., "Bash", "Read", "TodoWrite", "AskUserQuestion")
- `tool_input` - Full input parameters to the tool
- `tool_response` - Response from the tool

**TodoWrite tool_input example:**
```json
{
  "todos": [
    {"content": "Task 1", "status": "pending", "activeForm": "..."},
    {"content": "Task 2", "status": "in_progress", "activeForm": "..."}
  ]
}
```

## Stop Hook

**Stdin JSON structure:**
```json
{
  "session_id": "34dd35d1-709d-4d9a-890e-c313e8ae5417",
  "transcript_path": "/Users/nishadsingh/.claude/projects/-private-tmp/34dd35d1-709d-4d9a-890e-c313e8ae5417.jsonl",
  "cwd": "/private/tmp",
  "permission_mode": "default",
  "hook_event_name": "Stop",
  "stop_hook_active": false
}
```

**Note:** `stop_hook_active` may indicate error state - needs more testing.

## Environment Variables

Only generic Claude vars in env:
- `CLAUDE_PROJECT_DIR` - Project directory (same as `cwd` in stdin)
- `CLAUDE_CODE_ENTRYPOINT` - "cli"
- `CLAUDE_CODE_SSE_PORT` - Internal port

**TMUX_PANE** - Available if running in tmux (wasn't present in test since it was run from VS Code terminal).

## Hook Implementation

Hooks must:
1. Read JSON from stdin
2. Parse to extract session_id, tool_name, etc.
3. POST to daemon

Example:
```bash
#!/bin/bash
INPUT=$(cat)
curl -s -X POST http://localhost:7890/events \
  -H "Content-Type: application/json" \
  -d "$INPUT" || true
```

Since the stdin is already JSON with all the fields we need, we can forward it directly (or transform it slightly).
