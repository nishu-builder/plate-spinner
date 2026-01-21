import asyncio
import importlib.resources

from textual.app import App, ComposeResult
from textual.containers import VerticalScroll
from textual.widgets import Footer, Header, Static
from textual.binding import Binding

import httpx
import websockets

from ..config import load_config


async def play_sound(sound_name: str) -> None:
    if sound_name == "none":
        return
    try:
        ref = importlib.resources.files("plate_spinner.sounds") / f"{sound_name}.wav"
        with importlib.resources.as_file(ref) as path:
            if path.exists():
                await asyncio.create_subprocess_exec(
                    "afplay", str(path),
                    stdout=asyncio.subprocess.DEVNULL,
                    stderr=asyncio.subprocess.DEVNULL,
                )
    except OSError:
        pass


class SessionWidget(Static):
    def __init__(self, session: dict, index: int) -> None:
        self.session = session
        self.index = index
        super().__init__()

    def compose(self) -> ComposeResult:
        status = self.session["status"]
        folder = self.session["project_path"].rstrip("/").split("/")[-1]
        branch = self.session.get("git_branch") or ""
        todo = self.session.get("todo_progress") or ""
        summary = self.session.get("summary") or ""

        if branch:
            label = f"{folder}/{branch}"
        else:
            label = folder
        if len(label) > 25:
            label = label[:22] + "..."

        status_icons = {
            "starting": ".",
            "running": ">",
            "idle": "-",
            "awaiting_input": "?",
            "awaiting_approval": "!",
            "error": "X",
            "closed": "x",
        }
        icon = status_icons.get(status, " ")

        line = f"[{self.index}] {icon} {label:<25} {status}"
        if todo:
            line += f"  [{todo}]"
        if summary:
            line += f"  {summary}"
        yield Static(line)


class SessionGroup(Static):
    def __init__(self, title: str, sessions: list[dict], start_index: int) -> None:
        self.title = title
        self.sessions = sessions
        self.start_index = start_index
        super().__init__()

    def compose(self) -> ComposeResult:
        yield Static(f"\n{self.title}", classes="group-title")
        for i, session in enumerate(self.sessions):
            yield SessionWidget(session, self.start_index + i + 1)


class PlateSpinnerApp(App):
    CSS = """
    .group-title {
        color: $text-muted;
        text-style: bold;
    }
    SessionWidget {
        padding: 0 1;
    }
    """

    BINDINGS = [
        Binding("q", "quit", "Quit"),
        Binding("r", "refresh", "Refresh"),
        Binding("x", "dismiss_prompt", "Dismiss"),
        Binding("1", "jump(1)", "Jump 1", show=False),
        Binding("2", "jump(2)", "Jump 2", show=False),
        Binding("3", "jump(3)", "Jump 3", show=False),
        Binding("4", "jump(4)", "Jump 4", show=False),
        Binding("5", "jump(5)", "Jump 5", show=False),
        Binding("6", "jump(6)", "Jump 6", show=False),
        Binding("7", "jump(7)", "Jump 7", show=False),
        Binding("8", "jump(8)", "Jump 8", show=False),
        Binding("9", "jump(9)", "Jump 9", show=False),
    ]

    dismiss_mode: bool = False

    def __init__(self, daemon_url: str = "http://localhost:7890") -> None:
        super().__init__()
        self.daemon_url = daemon_url
        self.sessions: list[dict] = []
        self.display_order: list[dict] = []
        self.previous_statuses: dict[str, str] = {}
        self.config = load_config()

    def compose(self) -> ComposeResult:
        yield Header()
        yield VerticalScroll(id="main")
        yield Footer()

    async def on_mount(self) -> None:
        self.title = "Plate-Spinner"
        await self.action_refresh()
        self.run_worker(self.connect_websocket())

    async def connect_websocket(self) -> None:
        ws_url = self.daemon_url.replace("http://", "ws://") + "/ws"
        while True:
            try:
                async with websockets.connect(ws_url) as ws:
                    async for _ in ws:
                        await self.action_refresh()
            except (OSError, websockets.WebSocketException):
                pass
            await asyncio.sleep(2)

    async def action_refresh(self) -> None:
        try:
            async with httpx.AsyncClient() as client:
                response = await client.get(f"{self.daemon_url}/sessions")
                self.sessions = response.json()
        except httpx.RequestError:
            self.sessions = []

        for session in self.sessions:
            session_id = session["session_id"]
            current_status = session["status"]
            previous_status = self.previous_statuses.get(session_id)

            if previous_status == "running" and current_status != "running":
                if self.config.sounds.enabled:
                    sound_name = getattr(self.config.sounds, current_status, "none")
                    asyncio.create_task(play_sound(sound_name))

            self.previous_statuses[session_id] = current_status

        self.render_sessions()

    def render_sessions(self) -> None:
        main = self.query_one("#main")
        main.remove_children()

        if not self.sessions:
            main.mount(Static("\nNo active sessions.\n\nRun 'sp run' to start a tracked session."))
            self.sub_title = ""
            self.display_order = []
            return

        open_sessions = [s for s in self.sessions if s["status"] != "closed"]
        closed_sessions = [s for s in self.sessions if s["status"] == "closed"]

        # Sort open: needs attention first (not running), then running
        needs_attention = [s for s in open_sessions if s["status"] != "running"]
        running = [s for s in open_sessions if s["status"] == "running"]
        open_sorted = needs_attention + running

        self.display_order = open_sorted + closed_sessions

        attention_count = len(needs_attention)
        self.sub_title = f"{attention_count} need attention" if attention_count else ""

        idx = 0
        if open_sorted:
            main.mount(SessionGroup("OPEN", open_sorted, idx))
            idx += len(open_sorted)
        if closed_sessions:
            main.mount(SessionGroup("CLOSED", closed_sessions, idx))

    def action_jump(self, index: int) -> None:
        if self.dismiss_mode:
            self.dismiss_mode = False
            self.sub_title = f"{len([s for s in self.sessions if s['status'] in ('awaiting_input', 'awaiting_approval', 'error', 'idle')])} need attention" if self.sessions else ""
            asyncio.create_task(self._dismiss_session(index))
            return

        if index > len(self.display_order):
            self.notify("No session at that index", severity="warning")
            return

        session = self.display_order[index - 1]
        if session["status"] == "starting":
            self.notify("Session still starting", severity="warning")
            return
        self.exit(result=f"resume:{session['session_id']}:{session['project_path']}")

    def action_dismiss_prompt(self) -> None:
        if not self.display_order:
            self.notify("No sessions to dismiss", severity="warning")
            return
        self.dismiss_mode = True
        self.sub_title = "Press 1-9 to dismiss session, any other key to cancel"

    async def _dismiss_session(self, index: int) -> None:
        if index > len(self.display_order):
            self.notify("No session at that index", severity="warning")
            return

        session = self.display_order[index - 1]
        session_id = session["session_id"]

        try:
            async with httpx.AsyncClient() as client:
                response = await client.delete(f"{self.daemon_url}/sessions/{session_id}")
                if response.status_code == 200:
                    self.notify("Dismissed session")
                    await self.action_refresh()
                else:
                    self.notify("Failed to dismiss session", severity="error")
        except httpx.RequestError:
            self.notify("Failed to dismiss session", severity="error")

def run() -> str | None:
    app = PlateSpinnerApp()
    return app.run()
