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


STATUS_ICONS = {
    "starting": ".",
    "running": ">",
    "idle": "-",
    "awaiting_input": "?",
    "awaiting_approval": "!",
    "error": "X",
    "closed": "x",
}

STATUS_COLORS = {
    "starting": "dim",
    "running": "green",
    "idle": "cyan",
    "awaiting_input": "yellow",
    "awaiting_approval": "bright_magenta",
    "error": "red",
    "closed": "dim",
}


def format_session_line(session: dict, index: int, index_width: int = 1) -> str:
    status = session["status"]
    folder = session["project_path"].rstrip("/").split("/")[-1]
    branch = session.get("git_branch") or ""
    todo = session.get("todo_progress") or ""
    summary = session.get("summary") or ""

    if branch:
        label = f"{folder}/{branch}"
    else:
        label = folder
    if len(label) > 25:
        label = label[:22] + "..."

    icon = STATUS_ICONS.get(status, " ")
    color = STATUS_COLORS.get(status, "")
    status_padded = f"{status:<17}"
    colored_status = f"[{color}]{status_padded}[/]" if color else status_padded
    line = f"[{index:>{index_width}}] {icon} {label:<25} {colored_status}"
    if todo:
        line += f"  [{todo}]"
    if summary:
        line += f"  {summary}"
    return line


class SessionWidget(Static):
    def __init__(self, session: dict, index: int, index_width: int = 1, widget_id: str | None = None) -> None:
        self.session = session
        self.index = index
        self.index_width = index_width
        super().__init__(format_session_line(session, index, index_width), id=widget_id)

    def update_session(self, session: dict, index: int, index_width: int = 1) -> None:
        self.session = session
        self.index = index
        self.index_width = index_width
        self.update(format_session_line(session, index, index_width))


class GroupTitle(Static):
    pass


STATUSES = ["awaiting_input", "awaiting_approval", "error", "idle", "closed"]


class SoundSettingsScreen(ModalScreen["PlateSpinnerApp"]):
    BINDINGS = [
        Binding("escape", "dismiss", "Close"),
        Binding("up", "move_up", "Up", show=False),
        Binding("down", "move_down", "Down", show=False),
        Binding("left", "prev_sound", "Prev", show=False),
        Binding("right", "next_sound", "Next", show=False),
        Binding("space", "preview", "Preview", show=False),
        Binding("enter", "select", "Select", show=False),
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
            with Horizontal(classes="setting-row", id="row-save"):
                yield Static("Save and Close [esc]", classes="setting-label")

    def _update_selection(self) -> None:
        rows = ["toggle"] + STATUSES + ["save"]
        for i, row_id in enumerate(rows):
            row = self.query_one(f"#row-{row_id}")
            if i == self.selected_row:
                row.add_class("selected")
            else:
                row.remove_class("selected")

    def _get_current_row_id(self) -> str:
        rows = ["toggle"] + STATUSES + ["save"]
        return rows[self.selected_row]

    def action_move_up(self) -> None:
        if self.selected_row > 0:
            self.selected_row -= 1
            self._update_selection()

    def action_move_down(self) -> None:
        max_row = len(STATUSES) + 1  # +1 for save button
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
        if row_id != "toggle" and row_id != "save":
            sound = getattr(self.app.config.sounds, row_id, "none")
            if sound != "none":
                asyncio.create_task(play_sound(sound))

    def action_select(self) -> None:
        row_id = self._get_current_row_id()
        if row_id == "save":
            self.dismiss()
        elif row_id == "toggle":
            self._change_sound(1)
        else:
            self._change_sound(1)


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
    SessionWidget.selected {
        background: $boost;
    }
    """

    BINDINGS = [
        Binding("q", "quit", "Quit"),
        Binding("r", "refresh", "Refresh"),
        Binding("c", "toggle_closed", "Close/Open"),
        Binding("s", "sound_settings", "Sounds"),
        Binding("delete", "dismiss", "Dismiss"),
        Binding("backspace", "dismiss", "Dismiss", show=False),
        Binding("up", "move_up", "Up", show=False),
        Binding("down", "move_down", "Down", show=False),
        Binding("enter", "select", "Select", show=False),
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

    SYNC_INDICATOR = " ~"

    def __init__(self, daemon_url: str = "http://localhost:7890") -> None:
        super().__init__()
        self.daemon_url = daemon_url
        self.sessions: list[dict] = []
        self.display_order: list[dict] = []
        self.previous_statuses: dict[str, str] = {}
        self.config = load_config()
        self._sync_timer: asyncio.TimerHandle | None = None
        self._last_attention_count: int = -1
        self._render_lock = asyncio.Lock()
        self.selected_index: int = 0
        self._api_key_configured: bool | None = None
        self._hooks_installed: bool | None = None

    def compose(self) -> ComposeResult:
        yield Header()
        yield VerticalScroll(id="main")
        yield Footer()

    async def on_mount(self) -> None:
        self.title = "Plate-Spinner"
        if self.config.theme.name:
            self.theme = self.config.theme.name
        await self._fetch_daemon_status()
        await self.action_refresh()
        self.run_worker(self.connect_websocket())

    async def _fetch_daemon_status(self) -> None:
        try:
            async with httpx.AsyncClient() as client:
                response = await client.get(f"{self.daemon_url}/status")
                data = response.json()
                self._api_key_configured = data.get("api_key_configured", False)
                self._hooks_installed = data.get("hooks_installed", False)
        except httpx.RequestError:
            self._api_key_configured = None
            self._hooks_installed = None

    def watch_theme(self, theme: str) -> None:
        self.config.theme.name = theme
        save_config(self.config)

    def action_sound_settings(self) -> None:
        self.push_screen(SoundSettingsScreen())

    def _show_sync_indicator(self) -> None:
        if self._sync_timer:
            self._sync_timer.cancel()
        if not self.title.endswith(self.SYNC_INDICATOR):
            self.title = self.title + self.SYNC_INDICATOR
        self._sync_timer = asyncio.get_event_loop().call_later(
            0.3, self._hide_sync_indicator
        )

    def _hide_sync_indicator(self) -> None:
        if self.title.endswith(self.SYNC_INDICATOR):
            self.title = self.title[: -len(self.SYNC_INDICATOR)]
        self._sync_timer = None

    def _update_terminal_title(self, attention_count: int) -> None:
        if attention_count == self._last_attention_count:
            return
        self._last_attention_count = attention_count
        if attention_count > 0:
            title = f"Plate-Spinner ({attention_count})"
        else:
            title = "Plate-Spinner"
        try:
            with open("/dev/tty", "w") as tty:
                tty.write(f"\033]2;{title}\007")
                tty.flush()
        except OSError:
            pass

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
        self._show_sync_indicator()
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

        await self.render_sessions()

    async def render_sessions(self) -> None:
        async with self._render_lock:
            main = self.query_one("#main")

            if not self.sessions:
                if not main.query("Static#empty-message"):
                    await main.remove_children()
                    msg = "\nNo active sessions.\n\nRun 'sp run' to start a tracked session."
                    if self._hooks_installed is False:
                        msg += "\n\n[red]Hooks outdated or missing. Run 'sp install' to update.[/]"
                    if self._api_key_configured is False:
                        msg += "\n\n[yellow]Note: ANTHROPIC_API_KEY not set (summaries disabled)[/]"
                    await main.mount(Static(msg, id="empty-message"))
                self.sub_title = ""
                self.display_order = []
                self._update_terminal_title(0)
                return

            open_sessions = [s for s in self.sessions if s["status"] != "closed"]
            closed_sessions = [s for s in self.sessions if s["status"] == "closed"]

            needs_attention = [s for s in open_sessions if s["status"] != "running"]
            running = [s for s in open_sessions if s["status"] == "running"]
            open_sorted = needs_attention + running

            new_display_order = open_sorted + closed_sessions
            new_open_ids = [s["session_id"] for s in open_sorted]
            new_closed_ids = [s["session_id"] for s in closed_sessions]
            old_open_ids = [s["session_id"] for s in self.display_order if s["status"] != "closed"]
            old_closed_ids = [s["session_id"] for s in self.display_order if s["status"] == "closed"]

            attention_count = len(needs_attention)
            self.sub_title = f"{attention_count} need attention" if attention_count else ""
            self._update_terminal_title(attention_count)

            index_width = len(str(len(new_display_order)))

            same_grouping = new_open_ids == old_open_ids and new_closed_ids == old_closed_ids
            if same_grouping and not main.query("Static#empty-message"):
                for i, session in enumerate(new_display_order):
                    widget_id = f"session-{session['session_id']}"
                    try:
                        widget = main.query_one(f"#{widget_id}", SessionWidget)
                        widget.update_session(session, i + 1, index_width)
                    except Exception:
                        pass
                self.display_order = new_display_order
                self._update_selection()
                return

            await main.remove_children()
            self.display_order = new_display_order

            if self.selected_index >= len(self.display_order):
                self.selected_index = max(0, len(self.display_order) - 1)

            idx = 1
            if open_sorted:
                await main.mount(GroupTitle("\nOPEN", classes="group-title", id="group-open"))
                for session in open_sorted:
                    await main.mount(SessionWidget(session, idx, index_width, widget_id=f"session-{session['session_id']}"))
                    idx += 1
            if closed_sessions:
                await main.mount(GroupTitle("\nCLOSED", classes="group-title", id="group-closed"))
                for session in closed_sessions:
                    await main.mount(SessionWidget(session, idx, index_width, widget_id=f"session-{session['session_id']}"))
                    idx += 1

            self._update_selection()

    def _update_selection(self) -> None:
        main = self.query_one("#main")
        for i, session in enumerate(self.display_order):
            widget_id = f"session-{session['session_id']}"
            try:
                widget = main.query_one(f"#{widget_id}", SessionWidget)
                if i == self.selected_index:
                    widget.add_class("selected")
                else:
                    widget.remove_class("selected")
            except Exception:
                pass

    def action_move_up(self) -> None:
        if not self.display_order:
            return
        if self.selected_index > 0:
            self.selected_index -= 1
            self._update_selection()

    def action_move_down(self) -> None:
        if not self.display_order:
            return
        if self.selected_index < len(self.display_order) - 1:
            self.selected_index += 1
            self._update_selection()

    def action_select(self) -> None:
        if not self.display_order:
            return
        session = self.display_order[self.selected_index]
        if session["status"] == "starting":
            self.notify("Session still starting", severity="warning")
            return
        self.exit(result=f"resume:{session['session_id']}:{session['project_path']}")

    def action_dismiss(self) -> None:
        if not self.display_order:
            self.notify("No sessions to dismiss", severity="warning")
            return
        asyncio.create_task(self._dismiss_selected())

    def action_toggle_closed(self) -> None:
        if not self.display_order:
            self.notify("No sessions to toggle", severity="warning")
            return
        asyncio.create_task(self._toggle_selected())

    def action_jump(self, index: int) -> None:
        if index > len(self.display_order):
            self.notify("No session at that index", severity="warning")
            return
        self.selected_index = index - 1
        self._update_selection()
        self.action_select()

    async def _dismiss_selected(self) -> None:
        session = self.display_order[self.selected_index]
        session_id = session["session_id"]

        try:
            async with httpx.AsyncClient() as client:
                response = await client.delete(f"{self.daemon_url}/sessions/{session_id}")
                if response.status_code == 200:
                    self.notify("Dismissed session")
                    if self.selected_index >= len(self.display_order) - 1:
                        self.selected_index = max(0, self.selected_index - 1)
                    await self.action_refresh()
                else:
                    self.notify("Failed to dismiss session", severity="error")
        except httpx.RequestError:
            self.notify("Failed to dismiss session", severity="error")

    async def _toggle_selected(self) -> None:
        session = self.display_order[self.selected_index]
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
