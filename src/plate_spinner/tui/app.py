import asyncio
import importlib.resources

from textual.app import App, ComposeResult
from textual.binding import Binding
from textual.command import Provider, Hits, Hit, DiscoveryHit
from textual.containers import VerticalScroll
from textual.screen import ModalScreen
from textual.widgets import Footer, Header, Static

import httpx
import websockets

from ..config import load_config, save_config, get_available_sounds


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


STATUSES = ["awaiting_input", "awaiting_approval", "error", "idle", "closed"]


class SoundSettingsScreen(ModalScreen["PlateSpinnerApp"]):
    BINDINGS = [
        Binding("escape", "dismiss", "Close"),
        Binding("up", "move_up", "Up", show=False),
        Binding("down", "move_down", "Down", show=False),
        Binding("left", "prev_sound", "Prev", show=False),
        Binding("right", "next_sound", "Next", show=False),
        Binding("space", "preview", "Preview", show=False),
    ]

    CSS = """
    SoundSettingsScreen {
        align: center middle;
    }
    #sound-settings {
        width: 50;
        height: auto;
        max-height: 80%;
        border: solid $primary;
        background: $surface;
        padding: 1 2;
    }
    .setting-row {
        height: 1;
        padding: 0 1;
    }
    .setting-row.selected {
        background: $accent;
    }
    .setting-label {
        width: 20;
    }
    .setting-value {
        width: 1fr;
        text-align: right;
    }
    """

    def __init__(self) -> None:
        super().__init__()
        self.selected_row = 0
        self.sounds = get_available_sounds()

    def compose(self) -> ComposeResult:
        from textual.containers import Vertical, Horizontal
        with Vertical(id="sound-settings"):
            yield Static("Sound Settings", classes="title")
            yield Static("")
            with Horizontal(classes="setting-row selected", id="row-toggle"):
                yield Static("Enabled", classes="setting-label")
                enabled = "yes" if self.app.config.sounds.enabled else "no"
                yield Static(f"< {enabled} >", classes="setting-value", id="val-toggle")
            yield Static("")
            for status in STATUSES:
                label = status.replace("_", " ").title()
                current = getattr(self.app.config.sounds, status, "none")
                with Horizontal(classes="setting-row", id=f"row-{status}"):
                    yield Static(label, classes="setting-label")
                    yield Static(f"< {current} >", classes="setting-value", id=f"val-{status}")
            yield Static("")
            yield Static("[esc] save and close", classes="help")

    def _update_selection(self) -> None:
        rows = ["toggle"] + STATUSES
        for i, row_id in enumerate(rows):
            row = self.query_one(f"#row-{row_id}")
            if i == self.selected_row:
                row.add_class("selected")
            else:
                row.remove_class("selected")

    def _get_current_row_id(self) -> str:
        rows = ["toggle"] + STATUSES
        return rows[self.selected_row]

    def action_move_up(self) -> None:
        if self.selected_row > 0:
            self.selected_row -= 1
            self._update_selection()

    def action_move_down(self) -> None:
        max_row = len(STATUSES)
        if self.selected_row < max_row:
            self.selected_row += 1
            self._update_selection()

    def action_prev_sound(self) -> None:
        self._change_sound(-1)

    def action_next_sound(self) -> None:
        self._change_sound(1)

    def _change_sound(self, direction: int) -> None:
        row_id = self._get_current_row_id()
        if row_id == "toggle":
            self.app.config.sounds.enabled = not self.app.config.sounds.enabled
            enabled = "yes" if self.app.config.sounds.enabled else "no"
            self.query_one("#val-toggle", Static).update(f"< {enabled} >")
        else:
            current = getattr(self.app.config.sounds, row_id, "none")
            idx = self.sounds.index(current) if current in self.sounds else 0
            idx = (idx + direction) % len(self.sounds)
            new_sound = self.sounds[idx]
            setattr(self.app.config.sounds, row_id, new_sound)
            self.query_one(f"#val-{row_id}", Static).update(f"< {new_sound} >")
            if new_sound != "none":
                asyncio.create_task(play_sound(new_sound))
        save_config(self.app.config)

    def action_preview(self) -> None:
        row_id = self._get_current_row_id()
        if row_id != "toggle":
            sound = getattr(self.app.config.sounds, row_id, "none")
            if sound != "none":
                asyncio.create_task(play_sound(sound))


class SoundCommands(Provider):
    @property
    def app(self) -> "PlateSpinnerApp":
        return self.screen.app  # type: ignore[return-value]

    async def discover(self) -> Hits:
        yield DiscoveryHit(
            "Sound settings",
            self.action_open_settings,
            help="Configure notification sounds",
        )

    async def search(self, query: str) -> Hits:
        matcher = self.matcher(query)
        score = matcher.match("Sound settings")
        if score > 0:
            yield Hit(score, "Sound settings", self.action_open_settings, help="Configure notification sounds")

    async def action_open_settings(self) -> None:
        self.app.push_screen(SoundSettingsScreen())


class PlateSpinnerApp(App):
    COMMANDS = App.COMMANDS | {SoundCommands}

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
        Binding("shift+r", "toggle_closed_prompt", "Close/Open"),
        Binding("s", "sound_settings", "Sounds"),
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

    pending_action: str | None = None  # "dismiss" or "toggle"

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
        if self.config.theme.name:
            self.theme = self.config.theme.name
        await self.action_refresh()
        self.run_worker(self.connect_websocket())

    def watch_theme(self, theme: str) -> None:
        self.config.theme.name = theme
        save_config(self.config)

    def action_sound_settings(self) -> None:
        self.push_screen(SoundSettingsScreen())

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

    def _reset_attention_subtitle(self) -> None:
        count = len([s for s in self.sessions if s["status"] in ("awaiting_input", "awaiting_approval", "error", "idle")])
        self.sub_title = f"{count} need attention" if count else ""

    def _cancel_pending_action(self) -> None:
        if self.pending_action:
            self.pending_action = None
            self._reset_attention_subtitle()

    def on_key(self, event) -> None:
        if self.pending_action and event.key not in "123456789":
            self._cancel_pending_action()

    def action_jump(self, index: int) -> None:
        if self.pending_action == "dismiss":
            self.pending_action = None
            self._reset_attention_subtitle()
            asyncio.create_task(self._dismiss_session(index))
            return

        if self.pending_action == "toggle":
            self.pending_action = None
            self._reset_attention_subtitle()
            asyncio.create_task(self._toggle_session(index))
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
        self.pending_action = "dismiss"
        self.sub_title = "Press 1-9 to dismiss session, any other key to cancel"

    def action_toggle_closed_prompt(self) -> None:
        if not self.display_order:
            self.notify("No sessions to toggle", severity="warning")
            return
        self.pending_action = "toggle"
        self.sub_title = "Press 1-9 to close/reopen session, any other key to cancel"

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

    async def _toggle_session(self, index: int) -> None:
        if index > len(self.display_order):
            self.notify("No session at that index", severity="warning")
            return

        session = self.display_order[index - 1]
        session_id = session["session_id"]
        was_closed = session["status"] == "closed"

        try:
            async with httpx.AsyncClient() as client:
                response = await client.patch(f"{self.daemon_url}/sessions/{session_id}/toggle-closed")
                if response.status_code == 200:
                    action = "Reopened" if was_closed else "Closed"
                    self.notify(f"{action} session")
                    await self.action_refresh()
                else:
                    self.notify("Failed to toggle session", severity="error")
        except httpx.RequestError:
            self.notify("Failed to toggle session", severity="error")

def run() -> str | None:
    app = PlateSpinnerApp()
    return app.run()
