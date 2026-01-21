#!/bin/bash
INPUT=$(cat)

curl -s --connect-timeout 1 http://localhost:7890/health >/dev/null 2>&1 || exit 0

if command -v jq &>/dev/null; then
  PAYLOAD=$(echo "$INPUT" | jq -c '{
    session_id: .session_id,
    project_path: .cwd,
    event_type: "tool_start",
    tool_name: .tool_name,
    tool_params: .tool_input
  }')
else
  PAYLOAD="$INPUT"
fi

curl -s -X POST http://localhost:7890/events \
  -H "Content-Type: application/json" \
  -d "$PAYLOAD" >/dev/null 2>&1 || true
