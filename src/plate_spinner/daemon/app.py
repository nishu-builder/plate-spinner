from datetime import datetime, timezone

from fastapi import FastAPI

from .db import Database
from .models import HookEvent, SessionStatus


def create_app(db: Database) -> FastAPI:
    app = FastAPI(title="Plate-Spinner Daemon")

    @app.post("/events")
    async def post_event(event: HookEvent) -> dict:
        now = datetime.now(timezone.utc).isoformat()

        existing = db.execute(
            "SELECT session_id FROM sessions WHERE session_id = ?",
            (event.session_id,)
        ).fetchone()

        if event.event_type == "stop":
            status = SessionStatus.ERROR if event.error else SessionStatus.IDLE
        else:
            status = SessionStatus.from_tool(event.tool_name or "")

        if existing:
            db.execute(
                """UPDATE sessions SET
                   status = ?, last_event_type = ?, last_tool = ?,
                   tmux_pane = COALESCE(?, tmux_pane), updated_at = ?
                   WHERE session_id = ?""",
                (status.value, event.event_type, event.tool_name,
                 event.tmux_pane, now, event.session_id)
            )
        else:
            db.execute(
                """INSERT INTO sessions
                   (session_id, project_path, tmux_pane, status,
                    last_event_type, last_tool, created_at, updated_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?)""",
                (event.session_id, event.project_path, event.tmux_pane,
                 status.value, event.event_type, event.tool_name, now, now)
            )

        db.execute(
            """INSERT INTO events (session_id, event_type, payload, created_at)
               VALUES (?, ?, ?, ?)""",
            (event.session_id, event.event_type, event.model_dump_json(), now)
        )
        db.commit()

        return {"status": "ok"}

    @app.get("/sessions")
    async def get_sessions() -> list[dict]:
        rows = db.execute(
            """SELECT session_id, project_path, tmux_pane, status,
                      last_event_type, last_tool, created_at, updated_at
               FROM sessions ORDER BY updated_at DESC"""
        ).fetchall()
        return [dict(row) for row in rows]

    return app
