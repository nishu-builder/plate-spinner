from textual.app import App, ComposeResult
from textual.containers import VerticalScroll
from textual.widgets import Footer, Header, Static
from textual.binding import Binding

import httpx


class SessionWidget(Static):
    def __init__(self, session: dict, index: int) -> None:
        self.session = session
        self.index = index
        super().__init__()

    def compose(self) -> ComposeResult:
        status = self.session["status"]
        project = self.session["project_path"].rstrip("/").split("/")[-1]

        status_icons = {
            "running": ">",
            "idle": "-",
            "awaiting_input": "?",
            "awaiting_approval": "!",
            "error": "X",
        }
        icon = status_icons.get(status, " ")

        yield Static(f"[{self.index}] {icon} {project:<20} {status}")


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
        Binding("d", "dismiss", "Dismiss"),
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

    def __init__(self, daemon_url: str = "http://localhost:7890") -> None:
        super().__init__()
        self.daemon_url = daemon_url
        self.sessions: list[dict] = []
        self.display_order: list[dict] = []

    def compose(self) -> ComposeResult:
        yield Header()
        yield VerticalScroll(id="main")
        yield Footer()

    async def on_mount(self) -> None:
        self.title = "Plate-Spinner"
        await self.action_refresh()

    async def action_refresh(self) -> None:
        try:
            async with httpx.AsyncClient() as client:
                response = await client.get(f"{self.daemon_url}/sessions")
                self.sessions = response.json()
        except Exception:
            self.sessions = []

        self.render_sessions()

    def render_sessions(self) -> None:
        main = self.query_one("#main")
        main.remove_children()

        needs_attention = [s for s in self.sessions if s["status"] in
                          ("awaiting_input", "awaiting_approval", "error", "idle")]
        running = [s for s in self.sessions if s["status"] == "running"]

        self.display_order = needs_attention + running

        attention_count = len(needs_attention)
        self.sub_title = f"{attention_count} need attention" if attention_count else ""

        idx = 0
        if needs_attention:
            main.mount(SessionGroup("NEEDS ATTENTION", needs_attention, idx))
            idx += len(needs_attention)
        if running:
            main.mount(SessionGroup("RUNNING", running, idx))

    def action_jump(self, index: int) -> None:
        if index <= len(self.display_order):
            session = self.display_order[index - 1]
            pane = session.get("tmux_pane")
            if pane:
                import subprocess
                subprocess.run(["tmux", "select-pane", "-t", pane], check=False)

    def action_dismiss(self) -> None:
        pass


def run() -> None:
    app = PlateSpinnerApp()
    app.run()
