import json
from datetime import datetime, timezone

from fastapi import FastAPI, WebSocket, WebSocketDisconnect

from .db import Database
from .models import HookEvent, SessionStatus


class ConnectionManager:
    def __init__(self) -> None:
        self.connections: list[WebSocket] = []

    async def connect(self, websocket: WebSocket) -> None:
        await websocket.accept()
        self.connections.append(websocket)

    def disconnect(self, websocket: WebSocket) -> None:
        if websocket in self.connections:
            self.connections.remove(websocket)

    async def broadcast(self, message: dict) -> None:
        dead: list[WebSocket] = []
        for connection in self.connections:
            try:
                await connection.send_json(message)
            except Exception:
                dead.append(connection)
        for conn in dead:
            self.disconnect(conn)


def create_app(db: Database) -> FastAPI:
    app = FastAPI(title="Plate-Spinner Daemon")
    manager = ConnectionManager()

    @app.get("/health")
    async def health() -> dict:
        return {"status": "ok"}

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

        if event.tool_name == "TodoWrite" and event.tool_params:
            todos = event.tool_params.get("todos", [])
            if todos:
                todos_json = json.dumps(todos)
                db.execute(
                    """INSERT OR REPLACE INTO todos (session_id, todos_json, updated_at)
                       VALUES (?, ?, ?)""",
                    (event.session_id, todos_json, now)
                )

        db.execute(
            """INSERT INTO events (session_id, event_type, payload, created_at)
               VALUES (?, ?, ?, ?)""",
            (event.session_id, event.event_type, event.model_dump_json(), now)
        )
        db.commit()

        await manager.broadcast({"type": "session_update", "session_id": event.session_id})

        return {"status": "ok"}

    @app.get("/sessions")
    async def get_sessions() -> list[dict]:
        rows = db.execute(
            """SELECT s.session_id, s.project_path, s.tmux_pane, s.status,
                      s.last_event_type, s.last_tool, s.created_at, s.updated_at,
                      t.todos_json
               FROM sessions s
               LEFT JOIN todos t ON s.session_id = t.session_id
               ORDER BY s.updated_at DESC"""
        ).fetchall()

        result = []
        for row in rows:
            d = dict(row)
            todos_json = d.pop("todos_json", None)
            if todos_json:
                todos = json.loads(todos_json)
                completed = sum(1 for t in todos if t.get("status") == "completed")
                d["todo_progress"] = f"{completed}/{len(todos)}"
            else:
                d["todo_progress"] = None
            result.append(d)
        return result

    @app.websocket("/ws")
    async def websocket_endpoint(websocket: WebSocket) -> None:
        await manager.connect(websocket)
        try:
            while True:
                await websocket.receive_text()
        except WebSocketDisconnect:
            manager.disconnect(websocket)

    return app
