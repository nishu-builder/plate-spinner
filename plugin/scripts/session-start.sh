#!/bin/bash
INPUT=$(cat)

curl -s --connect-timeout 1 http://localhost:7890/health >/dev/null 2>&1 || exit 0

PAYLOAD=$(echo "$INPUT" | python3 -c '
import json, sys, subprocess
d = json.load(sys.stdin)
cwd = d.get("cwd", ".")
try:
    branch = subprocess.check_output(
        ["git", "-C", cwd, "rev-parse", "--abbrev-ref", "HEAD"],
        stderr=subprocess.DEVNULL
    ).decode().strip()
except:
    branch = None
print(json.dumps({
    "session_id": d.get("session_id"),
    "project_path": d.get("cwd"),
    "event_type": "session_start",
    "transcript_path": d.get("transcript_path"),
    "git_branch": branch if branch else None
}))
')

curl -s -X POST http://localhost:7890/events \
  -H "Content-Type: application/json" \
  -d "$PAYLOAD" >/dev/null 2>&1 || true
