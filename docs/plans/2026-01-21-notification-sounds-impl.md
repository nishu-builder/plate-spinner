# Notification Sounds Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add configurable notification sounds when sessions need attention, with a config system for sharing preferences.

**Architecture:** Config module with Pydantic models persists to TOML. TUI tracks session state changes and plays sounds via afplay. Textual command palette extended with custom providers for sound/theme selection. CLI gets config subcommand for import/export.

**Tech Stack:** Pydantic, toml (stdlib), importlib.resources, Textual command palette, asyncio subprocess

---

### Task 1: Create Config Module

**Files:**
- Create: `src/plate_spinner/config.py`
- Test: `tests/test_config.py`

**Step 1: Write the failing test for config defaults**

```python
# tests/test_config.py
import tempfile
from pathlib import Path

from plate_spinner.config import Config, SoundsConfig, load_config, get_config_path


def test_config_defaults():
    config = Config()
    assert config.sounds.enabled is True
    assert config.sounds.awaiting_input == "chime"
    assert config.sounds.awaiting_approval == "bell"
    assert config.sounds.error == "alert"
    assert config.sounds.idle == "pop"
    assert config.sounds.closed == "none"
    assert config.theme.name == "textual-dark"
```

**Step 2: Run test to verify it fails**

Run: `pytest tests/test_config.py::test_config_defaults -v`
Expected: FAIL with ModuleNotFoundError

**Step 3: Write minimal implementation**

```python
# src/plate_spinner/config.py
from pathlib import Path
from typing import Literal

from pydantic import BaseModel


SoundName = Literal["chime", "bell", "pop", "ping", "alert", "none"]


class SoundsConfig(BaseModel):
    enabled: bool = True
    awaiting_input: SoundName = "chime"
    awaiting_approval: SoundName = "bell"
    error: SoundName = "alert"
    idle: SoundName = "pop"
    closed: SoundName = "none"


class ThemeConfig(BaseModel):
    name: str = "textual-dark"


class Config(BaseModel):
    sounds: SoundsConfig = SoundsConfig()
    theme: ThemeConfig = ThemeConfig()


def get_config_path() -> Path:
    return Path.home() / ".config" / "plate-spinner" / "config.toml"


def load_config() -> Config:
    path = get_config_path()
    if not path.exists():
        return Config()
    try:
        import tomllib
        data = tomllib.loads(path.read_text())
        return Config.model_validate(data)
    except Exception:
        return Config()


def save_config(config: Config) -> None:
    path = get_config_path()
    path.parent.mkdir(parents=True, exist_ok=True)

    lines = ["[sounds]"]
    lines.append(f"enabled = {str(config.sounds.enabled).lower()}")
    lines.append(f'awaiting_input = "{config.sounds.awaiting_input}"')
    lines.append(f'awaiting_approval = "{config.sounds.awaiting_approval}"')
    lines.append(f'error = "{config.sounds.error}"')
    lines.append(f'idle = "{config.sounds.idle}"')
    lines.append(f'closed = "{config.sounds.closed}"')
    lines.append("")
    lines.append("[theme]")
    lines.append(f'name = "{config.theme.name}"')

    path.write_text("\n".join(lines) + "\n")
```

**Step 4: Run test to verify it passes**

Run: `pytest tests/test_config.py::test_config_defaults -v`
Expected: PASS

**Step 5: Write test for save/load roundtrip**

```python
def test_config_save_and_load(tmp_path, monkeypatch):
    config_path = tmp_path / "config.toml"
    monkeypatch.setattr("plate_spinner.config.get_config_path", lambda: config_path)

    from plate_spinner.config import save_config, load_config, Config, SoundsConfig

    config = Config(sounds=SoundsConfig(awaiting_input="ping", enabled=False))
    save_config(config)

    loaded = load_config()
    assert loaded.sounds.awaiting_input == "ping"
    assert loaded.sounds.enabled is False
```

**Step 6: Run test to verify it passes**

Run: `pytest tests/test_config.py::test_config_save_and_load -v`
Expected: PASS

**Step 7: Commit**

```bash
git add src/plate_spinner/config.py tests/test_config.py
git commit -m "Add config module with TOML persistence"
```

---

### Task 2: Add Sound Files

**Files:**
- Create: `src/plate_spinner/sounds/__init__.py`
- Create: `src/plate_spinner/sounds/chime.wav`
- Create: `src/plate_spinner/sounds/bell.wav`
- Create: `src/plate_spinner/sounds/pop.wav`
- Create: `src/plate_spinner/sounds/ping.wav`
- Create: `src/plate_spinner/sounds/alert.wav`

**Step 1: Create sounds directory and __init__.py**

```bash
mkdir -p src/plate_spinner/sounds
touch src/plate_spinner/sounds/__init__.py
```

**Step 2: Download CC0 notification sounds**

Download 5 short notification sounds from freesound.org (CC0 tag) or Pixabay:
- https://freesound.org/browse/tags/cc0/
- https://pixabay.com/sound-effects/search/notification/

Save as WAV files, rename to: chime.wav, bell.wav, pop.wav, ping.wav, alert.wav

Each file should be <100KB and ~0.5-1s duration.

**Step 3: Verify sounds load via importlib.resources**

```python
# Quick test in Python REPL
import importlib.resources
path = importlib.resources.files("plate_spinner.sounds") / "chime.wav"
print(path, path.is_file())
```

**Step 4: Commit**

```bash
git add src/plate_spinner/sounds/
git commit -m "Add notification sound files"
```

---

### Task 3: Add Sound Playback Utility

**Files:**
- Modify: `src/plate_spinner/config.py`
- Test: `tests/test_config.py`

**Step 1: Write failing test for get_sound_path**

```python
def test_get_sound_path():
    from plate_spinner.config import get_sound_path

    path = get_sound_path("chime")
    assert path is not None
    assert path.name == "chime.wav"
    assert path.exists()


def test_get_sound_path_none_returns_none():
    from plate_spinner.config import get_sound_path

    path = get_sound_path("none")
    assert path is None
```

**Step 2: Run test to verify it fails**

Run: `pytest tests/test_config.py::test_get_sound_path -v`
Expected: FAIL with ImportError

**Step 3: Add get_sound_path and get_available_sounds to config.py**

```python
import importlib.resources


def get_sound_path(name: str) -> Path | None:
    if name == "none":
        return None
    try:
        ref = importlib.resources.files("plate_spinner.sounds") / f"{name}.wav"
        with importlib.resources.as_file(ref) as path:
            return Path(path) if path.exists() else None
    except Exception:
        return None


def get_available_sounds() -> list[str]:
    return ["chime", "bell", "pop", "ping", "alert", "none"]
```

**Step 4: Run tests**

Run: `pytest tests/test_config.py -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/plate_spinner/config.py tests/test_config.py
git commit -m "Add sound path utilities"
```

---

### Task 4: Add CLI Config Commands

**Files:**
- Modify: `src/plate_spinner/cli.py`
- Test: `tests/test_cli.py` (new)

**Step 1: Write failing test for config path command**

```python
# tests/test_cli.py
import subprocess
import sys


def test_config_path_command():
    result = subprocess.run(
        [sys.executable, "-m", "plate_spinner.cli", "config", "path"],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0
    assert ".config/plate-spinner/config.toml" in result.stdout
```

**Step 2: Run test to verify it fails**

Run: `pytest tests/test_cli.py::test_config_path_command -v`
Expected: FAIL

**Step 3: Add config subparser to cli.py**

Add to `main()` after other subparsers:

```python
config_parser = subparsers.add_parser("config", help="Manage configuration")
config_subparsers = config_parser.add_subparsers(dest="config_command")
config_subparsers.add_parser("path", help="Print config file path")
config_subparsers.add_parser("export", help="Export config to stdout")
import_parser = config_subparsers.add_parser("import", help="Import config from file")
import_parser.add_argument("file", help="Config file to import")
```

Add command handlers:

```python
def cmd_config(args: argparse.Namespace) -> None:
    from .config import get_config_path, load_config, save_config, Config

    if args.config_command == "path":
        print(get_config_path())
    elif args.config_command == "export":
        config = load_config()
        path = get_config_path()
        if path.exists():
            print(path.read_text())
        else:
            save_config(config)
            print(path.read_text())
    elif args.config_command == "import":
        import tomllib
        data = tomllib.loads(Path(args.file).read_text())
        config = Config.model_validate(data)
        save_config(config)
        print(f"Imported config from {args.file}")
    else:
        print(f"Config path: {get_config_path()}")
```

Add to command dispatch:

```python
elif args.command == "config":
    cmd_config(args)
```

**Step 4: Run test**

Run: `pytest tests/test_cli.py::test_config_path_command -v`
Expected: PASS

**Step 5: Write test for export command**

```python
def test_config_export_command():
    result = subprocess.run(
        [sys.executable, "-m", "plate_spinner.cli", "config", "export"],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0
    assert "[sounds]" in result.stdout
    assert "enabled = true" in result.stdout
```

**Step 6: Run test**

Run: `pytest tests/test_cli.py::test_config_export_command -v`
Expected: PASS

**Step 7: Commit**

```bash
git add src/plate_spinner/cli.py tests/test_cli.py
git commit -m "Add config CLI commands"
```

---

### Task 5: Add Sound Playback to TUI

**Files:**
- Modify: `src/plate_spinner/tui/app.py`

**Step 1: Add play_sound function**

Add at top of file:

```python
import asyncio
import importlib.resources
from pathlib import Path

from ..config import load_config, get_sound_path


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
    except Exception:
        pass
```

**Step 2: Track previous session states in PlateSpinnerApp**

Add to `__init__`:

```python
self.previous_statuses: dict[str, str] = {}
self.config = load_config()
```

**Step 3: Add status change detection in action_refresh**

Modify `action_refresh` to detect status changes:

```python
async def action_refresh(self) -> None:
    try:
        async with httpx.AsyncClient() as client:
            response = await client.get(f"{self.daemon_url}/sessions")
            self.sessions = response.json()
    except Exception:
        self.sessions = []

    # Check for status changes
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
```

**Step 4: Manually test**

Run `sp tui` in one terminal. In another, create a session that triggers awaiting_input (e.g., via AskUserQuestion tool). Verify sound plays.

**Step 5: Commit**

```bash
git add src/plate_spinner/tui/app.py
git commit -m "Add sound notifications on status change"
```

---

### Task 6: Add Command Palette Providers

**Files:**
- Modify: `src/plate_spinner/tui/app.py`

**Step 1: Add imports for command palette**

```python
from textual.command import Provider, Hits, Hit, DiscoveryHit
```

**Step 2: Create SoundCommands provider**

```python
class SoundCommands(Provider):
    @property
    def app(self) -> "PlateSpinnerApp":
        return self.screen.app

    async def discover(self) -> Hits:
        for status in ["awaiting_input", "awaiting_approval", "error", "idle", "closed"]:
            label = status.replace("_", " ").title()
            yield DiscoveryHit(
                f"Set sound: {label}",
                self.action_select_sound,
                status,
                help=f"Choose notification sound for {label}",
            )
        yield DiscoveryHit(
            "Toggle sounds",
            self.action_toggle_sounds,
            help="Enable or disable all notification sounds",
        )

    async def search(self, query: str) -> Hits:
        matcher = self.matcher(query)
        for status in ["awaiting_input", "awaiting_approval", "error", "idle", "closed"]:
            label = status.replace("_", " ").title()
            command = f"Set sound: {label}"
            score = matcher.match(command)
            if score > 0:
                yield Hit(score, command, self.action_select_sound, status, help=f"Choose sound for {label}")

        score = matcher.match("Toggle sounds")
        if score > 0:
            enabled = self.app.config.sounds.enabled
            state = "on" if enabled else "off"
            yield Hit(score, f"Toggle sounds (currently {state})", self.action_toggle_sounds)

    async def action_toggle_sounds(self) -> None:
        from ..config import save_config
        self.app.config.sounds.enabled = not self.app.config.sounds.enabled
        save_config(self.app.config)
        state = "enabled" if self.app.config.sounds.enabled else "disabled"
        self.app.notify(f"Sounds {state}")

    async def action_select_sound(self, status: str) -> None:
        self.app.push_screen(SoundPickerScreen(status))
```

**Step 3: Create SoundPickerScreen**

```python
from textual.screen import ModalScreen
from textual.widgets import OptionList
from textual.widgets.option_list import Option


class SoundPickerScreen(ModalScreen):
    BINDINGS = [("escape", "dismiss", "Cancel")]

    def __init__(self, status: str) -> None:
        super().__init__()
        self.status = status

    def compose(self) -> ComposeResult:
        from ..config import get_available_sounds
        sounds = get_available_sounds()
        current = getattr(self.app.config.sounds, self.status, "none")
        options = [Option(f"{'* ' if s == current else '  '}{s}", id=s) for s in sounds]
        yield OptionList(*options, id="sound-picker")

    def on_option_list_option_selected(self, event: OptionList.OptionSelected) -> None:
        sound_name = event.option.id
        setattr(self.app.config.sounds, self.status, sound_name)
        from ..config import save_config
        save_config(self.app.config)

        if sound_name != "none":
            asyncio.create_task(play_sound(sound_name))

        label = self.status.replace("_", " ").title()
        self.app.notify(f"{label} sound set to {sound_name}")
        self.dismiss()
```

**Step 4: Register commands in PlateSpinnerApp**

```python
class PlateSpinnerApp(App):
    COMMANDS = App.COMMANDS | {SoundCommands}
```

**Step 5: Manually test**

Run `sp tui`, press Ctrl+P, type "sound", select an option, verify sound preview plays.

**Step 6: Commit**

```bash
git add src/plate_spinner/tui/app.py
git commit -m "Add command palette for sound selection"
```

---

### Task 7: Apply Theme from Config on Startup

**Files:**
- Modify: `src/plate_spinner/tui/app.py`

**Step 1: Apply theme in on_mount**

Add to `on_mount`:

```python
if self.config.theme.name:
    self.theme = self.config.theme.name
```

**Step 2: Save theme changes**

Override theme setter or use watch_theme:

```python
def watch_theme(self, theme: str) -> None:
    from ..config import save_config
    self.config.theme.name = theme
    save_config(self.config)
```

**Step 3: Manually test**

Run `sp tui`, Ctrl+P, change theme, restart TUI, verify theme persists.

**Step 4: Commit**

```bash
git add src/plate_spinner/tui/app.py
git commit -m "Persist theme selection to config"
```

---

## Sound Sources

Download CC0/public domain sounds from:
- [Freesound CC0](https://freesound.org/browse/tags/cc0/)
- [Pixabay Notifications](https://pixabay.com/sound-effects/search/notification/)

Normalize volume levels with: `ffmpeg -i input.wav -af "loudnorm=I=-16:TP=-1.5:LRA=11" output.wav`
