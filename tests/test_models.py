from plate_spinner.daemon.models import HookEvent, Session, SessionStatus


def test_hook_event_from_dict():
    data = {
        "session_id": "abc123",
        "project_path": "/path/to/project",
        "event_type": "tool_call",
        "tool_name": "AskUserQuestion",
        "tmux_pane": "%5",
    }
    event = HookEvent(**data)
    assert event.session_id == "abc123"
    assert event.tool_name == "AskUserQuestion"


def test_session_status_from_tool():
    assert SessionStatus.from_tool("AskUserQuestion") == SessionStatus.AWAITING_INPUT
    assert SessionStatus.from_tool("ExitPlanMode") == SessionStatus.AWAITING_APPROVAL
    assert SessionStatus.from_tool("Read") == SessionStatus.RUNNING
