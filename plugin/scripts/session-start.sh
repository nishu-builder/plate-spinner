#!/bin/bash
INPUT=$(cat)

curl -s --connect-timeout 1 http://localhost:7890/health >/dev/null 2>&1 || exit 0

GIT_BRANCH=$(git -C "$(echo "$INPUT" | jq -r '.cwd // "."')" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

if command -v jq &>/dev/null; then
  PAYLOAD=$(echo "$INPUT" | jq -c --arg branch "$GIT_BRANCH" '{
    session_id: .session_id,
    project_path: .cwd,
    event_type: "session_start",
    transcript_path: .transcript_path,
    git_branch: (if $branch == "" then null else $branch end)
  }')
else
  PAYLOAD="$INPUT"
fi

curl -s -X POST http://localhost:7890/events \
  -H "Content-Type: application/json" \
  -d "$PAYLOAD" >/dev/null 2>&1 || true
