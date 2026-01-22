#!/bin/bash
INPUT=$(cat)

curl -s --connect-timeout 1 http://localhost:7890/health >/dev/null 2>&1 || exit 0

PAYLOAD=$(echo "$INPUT" | python3 -c '
import json, sys
d = json.load(sys.stdin)
print(json.dumps({
    "session_id": d.get("session_id"),
    "project_path": d.get("cwd"),
    "event_type": "tool_start",
    "tool_name": d.get("tool_name"),
    "tool_params": d.get("tool_input")
}))
')

curl -s -X POST http://localhost:7890/events \
  -H "Content-Type: application/json" \
  -d "$PAYLOAD" >/dev/null 2>&1 || true
