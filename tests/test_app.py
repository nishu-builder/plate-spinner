import tempfile
from pathlib import Path

from fastapi.testclient import TestClient

from plate_spinner.daemon.app import create_app
from plate_spinner.daemon.db import Database


def test_post_event_creates_session():
    with tempfile.TemporaryDirectory() as tmpdir:
        db = Database(Path(tmpdir) / "test.db")
        app = create_app(db)
        client = TestClient(app)

        response = client.post("/events", json={
            "session_id": "abc123",
            "project_path": "/path/to/project",
            "event_type": "tool_call",
            "tool_name": "Read",
        })
        assert response.status_code == 200

        sessions = client.get("/sessions").json()
        assert len(sessions) == 1
        assert sessions[0]["session_id"] == "abc123"
        assert sessions[0]["status"] == "running"

        db.close()


def test_ask_user_question_sets_awaiting_input():
    with tempfile.TemporaryDirectory() as tmpdir:
        db = Database(Path(tmpdir) / "test.db")
        app = create_app(db)
        client = TestClient(app)

        client.post("/events", json={
            "session_id": "abc123",
            "project_path": "/path/to/project",
            "event_type": "tool_call",
            "tool_name": "AskUserQuestion",
        })

        sessions = client.get("/sessions").json()
        assert sessions[0]["status"] == "awaiting_input"

        db.close()


def test_health_endpoint():
    with tempfile.TemporaryDirectory() as tmpdir:
        db = Database(Path(tmpdir) / "test.db")
        app = create_app(db)
        client = TestClient(app)

        response = client.get("/health")
        assert response.status_code == 200
        assert response.json() == {"status": "ok"}

        db.close()
