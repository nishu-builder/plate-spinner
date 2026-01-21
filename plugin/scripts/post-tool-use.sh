#!/bin/bash
# Read JSON from stdin
INPUT=$(cat)

# Check if daemon is running
curl -s --connect-timeout 1 http://localhost:7890/health >/dev/null 2>&1 || exit 0

# Transform stdin JSON to match HookEvent model
# stdin has: session_id, cwd (as project_path), hook_event_name, tool_name, tool_input
# HookEvent needs: session_id, project_path, event_type, tool_name, tool_params, tmux_pane

if command -v jq &>/dev/null; then
  PAYLOAD=$(echo "$INPUT" | jq -c '{
    session_id: .session_id,
    project_path: .cwd,
    event_type: "tool_call",
    tool_name: .tool_name,
    tool_params: .tool_input,
    tmux_pane: (env.TMUX_PANE // null)
  }')
else
  # Fallback: forward as-is, daemon should handle
  PAYLOAD="$INPUT"
fi

curl -s -X POST http://localhost:7890/events \
  -H "Content-Type: application/json" \
  -d "$PAYLOAD" >/dev/null 2>&1 || true
