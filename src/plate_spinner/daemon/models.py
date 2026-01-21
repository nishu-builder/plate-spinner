from datetime import datetime, timezone
from enum import Enum

from pydantic import BaseModel, Field


class SessionStatus(str, Enum):
    RUNNING = "running"
    IDLE = "idle"
    AWAITING_INPUT = "awaiting_input"
    AWAITING_APPROVAL = "awaiting_approval"
    ERROR = "error"

    @classmethod
    def from_tool(cls, tool_name: str) -> "SessionStatus":
        if tool_name == "AskUserQuestion":
            return cls.AWAITING_INPUT
        if tool_name == "ExitPlanMode":
            return cls.AWAITING_APPROVAL
        return cls.RUNNING


class HookEvent(BaseModel):
    session_id: str
    project_path: str
    event_type: str
    tool_name: str | None = None
    tool_params: dict | None = None
    tmux_pane: str | None = None
    error: str | None = None
    timestamp: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))


class Session(BaseModel):
    session_id: str
    project_path: str
    tmux_pane: str | None = None
    status: SessionStatus = SessionStatus.RUNNING
    last_event_type: str | None = None
    last_tool: str | None = None
    created_at: datetime
    updated_at: datetime

    @property
    def project_name(self) -> str:
        return self.project_path.rstrip("/").split("/")[-1]

    @property
    def needs_attention(self) -> bool:
        return self.status in (
            SessionStatus.AWAITING_INPUT,
            SessionStatus.AWAITING_APPROVAL,
            SessionStatus.IDLE,
            SessionStatus.ERROR,
        )
