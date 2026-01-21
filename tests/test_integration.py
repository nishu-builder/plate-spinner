import subprocess
import tempfile
import time
from pathlib import Path

import httpx
import pytest


@pytest.fixture
def isolated_daemon():
    """Start a daemon with a fresh database in a temp directory."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "state.db"
        env = {
            "PATH": subprocess.os.environ.get("PATH", ""),
            "HOME": subprocess.os.environ.get("HOME", ""),
        }

        proc = subprocess.Popen(
            ["uv", "run", "python", "-c", f"""
import uvicorn
from plate_spinner.daemon.app import create_app
from plate_spinner.daemon.db import Database
from pathlib import Path

db = Database(Path("{db_path}"))
app = create_app(db)
uvicorn.run(app, host="127.0.0.1", port=17890, log_level="warning")
"""],
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

        for _ in range(20):
            try:
                httpx.get("http://localhost:17890/health", timeout=0.5)
                break
            except Exception:
                time.sleep(0.25)

        yield "http://localhost:17890"

        proc.terminate()
        proc.wait(timeout=5)


def test_pre_tool_use_sets_running(isolated_daemon):
    """PreToolUse hook should set status to running."""
    response = httpx.post(f"{isolated_daemon}/events", json={
        "session_id": "test-001",
        "project_path": "/tmp/project",
        "event_type": "tool_start",
        "tool_name": "Bash",
    })
    assert response.status_code == 200

    sessions = httpx.get(f"{isolated_daemon}/sessions").json()
    assert len(sessions) == 1
    assert sessions[0]["status"] == "running"


def test_post_tool_use_updates_status(isolated_daemon):
    """PostToolUse hook should update status based on tool."""
    # First, tool starts
    httpx.post(f"{isolated_daemon}/events", json={
        "session_id": "test-001",
        "project_path": "/tmp/project",
        "event_type": "tool_start",
        "tool_name": "AskUserQuestion",
    })

    sessions = httpx.get(f"{isolated_daemon}/sessions").json()
    assert sessions[0]["status"] == "running"

    # Then tool completes
    httpx.post(f"{isolated_daemon}/events", json={
        "session_id": "test-001",
        "project_path": "/tmp/project",
        "event_type": "tool_call",
        "tool_name": "AskUserQuestion",
    })

    sessions = httpx.get(f"{isolated_daemon}/sessions").json()
    assert sessions[0]["status"] == "awaiting_input"


def test_stop_sets_idle(isolated_daemon):
    """Stop event should set status to idle."""
    httpx.post(f"{isolated_daemon}/events", json={
        "session_id": "test-001",
        "project_path": "/tmp/project",
        "event_type": "tool_start",
        "tool_name": "Bash",
    })

    httpx.post(f"{isolated_daemon}/events", json={
        "session_id": "test-001",
        "project_path": "/tmp/project",
        "event_type": "stop",
    })

    sessions = httpx.get(f"{isolated_daemon}/sessions").json()
    assert sessions[0]["status"] == "idle"


def test_hook_script_transformation():
    """Test that hook scripts properly transform input."""
    import json

    hook_input = json.dumps({
        "session_id": "abc123",
        "cwd": "/path/to/project",
        "tool_name": "Read",
        "tool_input": {"file": "test.txt"},
    })

    result = subprocess.run(
        ["bash", "-c", f'''
        echo '{hook_input}' | jq -c '{{
            session_id: .session_id,
            project_path: .cwd,
            event_type: "tool_start",
            tool_name: .tool_name,
            tool_params: .tool_input
        }}'
        '''],
        capture_output=True,
        text=True,
        env={"PATH": subprocess.os.environ.get("PATH", "")},
    )

    output = json.loads(result.stdout.strip())
    assert output["session_id"] == "abc123"
    assert output["project_path"] == "/path/to/project"
    assert output["event_type"] == "tool_start"
    assert output["tool_name"] == "Read"
    assert output["tool_params"] == {"file": "test.txt"}
