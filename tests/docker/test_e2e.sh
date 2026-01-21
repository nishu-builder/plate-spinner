#!/bin/bash
set -e

echo "=== E2E Test: Plate-Spinner on Linux ==="

# Start daemon in background
echo "Starting daemon..."
uv run sp daemon &
DAEMON_PID=$!
sleep 2

# Verify daemon is running
curl -s http://localhost:7890/health | grep -q "ok" || { echo "FAIL: Daemon not healthy"; exit 1; }
echo "OK: Daemon is running"

# Test 1: Simulate pre-tool-use hook
echo "Test 1: PreToolUse hook..."
echo '{"session_id":"e2e-test-001","cwd":"/tmp/project","tool_name":"Bash","tool_input":{"command":"ls"}}' | \
  ~/.plate-spinner/hooks/pre-tool-use.sh

RESULT=$(curl -s http://localhost:7890/sessions)
echo "$RESULT" | grep -q '"status": "running"' || echo "$RESULT" | grep -q '"status":"running"' || { echo "FAIL: Status should be running"; echo "$RESULT"; exit 1; }
echo "OK: PreToolUse sets running status"

# Test 2: Simulate post-tool-use hook (AskUserQuestion)
echo "Test 2: PostToolUse with AskUserQuestion..."
echo '{"session_id":"e2e-test-001","cwd":"/tmp/project","tool_name":"AskUserQuestion","tool_input":{}}' | \
  ~/.plate-spinner/hooks/post-tool-use.sh

RESULT=$(curl -s http://localhost:7890/sessions)
echo "$RESULT" | grep -q '"status": "awaiting_input"' || echo "$RESULT" | grep -q '"status":"awaiting_input"' || { echo "FAIL: Status should be awaiting_input"; echo "$RESULT"; exit 1; }
echo "OK: AskUserQuestion sets awaiting_input status"

# Test 3: Simulate stop hook
echo "Test 3: Stop hook..."
echo '{"session_id":"e2e-test-001","cwd":"/tmp/project"}' | \
  ~/.plate-spinner/hooks/stop.sh

RESULT=$(curl -s http://localhost:7890/sessions)
echo "$RESULT" | grep -q '"status": "idle"' || echo "$RESULT" | grep -q '"status":"idle"' || { echo "FAIL: Status should be idle"; echo "$RESULT"; exit 1; }
echo "OK: Stop sets idle status"

# Cleanup
kill $DAEMON_PID 2>/dev/null || true

echo ""
echo "=== All E2E tests passed! ==="
