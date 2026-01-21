# Notification Sounds & Config System

## Overview

Add notification sounds to the TUI when sessions need attention, with per-status sound configuration and a shareable config file.

## Sound System

### Sound Files

WAV files stored in `src/plate_spinner/sounds/`:
- `chime.wav` - gentle chime
- `bell.wav` - soft bell
- `pop.wav` - quick pop
- `ping.wav` - short ping
- `alert.wav` - slightly more urgent tone

Files should be <100KB each, ~0.5-1s duration, volume-normalized.

### Trigger Logic

In the TUI, track previous status per session. On WebSocket update:
- Compare old vs new status
- If session was `running` and is now anything else, play the sound configured for the new status

### Playback

```python
async def play_sound(sound_name: str) -> None:
    if sound_name == "none":
        return
    sound_path = importlib.resources.files("plate_spinner.sounds") / f"{sound_name}.wav"
    asyncio.create_task(asyncio.subprocess.create_subprocess_exec("afplay", str(sound_path)))
```

## Config System

### Location

`~/.config/plate-spinner/config.toml`

### Structure

```toml
[sounds]
enabled = true
awaiting_input = "chime"
awaiting_approval = "bell"
error = "alert"
idle = "pop"
closed = "none"

[theme]
name = "textual-dark"
```

### Config Module

`src/plate_spinner/config.py`:
- `Config` - Pydantic model
- `load_config() -> Config` - loads from file, returns defaults if missing
- `save_config(config: Config) -> None` - writes to file

### CLI Commands

```bash
sp config export > my-config.toml      # prints config to stdout
sp config import my-config.toml        # replaces config from file
sp config path                         # prints config file path
```

## TUI Command Palette Integration

Textual's built-in command palette (Ctrl+P) already provides theme switching.

### Custom Commands

Add via `Provider` subclass:
- "Set sound: awaiting input" - pick sound for awaiting_input status
- "Set sound: awaiting approval" - pick sound for awaiting_approval status
- "Set sound: error" - pick sound for error status
- "Set sound: idle" - pick sound for idle status
- "Set sound: closed" - pick sound for closed status
- "Toggle all sounds" - quick enable/disable

Sound selection shows available sounds and plays a preview on select.

### Implementation

```python
class SoundCommands(Provider):
    async def search(self, query: str) -> Hits:
        for sound in get_available_sounds():
            if query.lower() in sound.lower():
                yield Hit(sound, self.set_sound, sound)

    async def set_sound(self, sound: str) -> None:
        config = load_config()
        config.sounds.name = sound
        save_config(config)
        play_sound(sound)  # preview

class PlateSpinnerApp(App):
    COMMANDS = App.COMMANDS | {SoundCommands}
```

## Package Structure

```
src/plate_spinner/
├── sounds/
│   ├── __init__.py
│   ├── chime.wav
│   ├── bell.wav
│   ├── pop.wav
│   ├── ping.wav
│   └── alert.wav
├── config.py
├── cli.py          # add config subcommands
├── tui/
│   └── app.py      # add sound triggers and command providers
...
```

## Default Config

```toml
[sounds]
enabled = true
awaiting_input = "chime"
awaiting_approval = "bell"
error = "alert"
idle = "pop"
closed = "none"

[theme]
name = "textual-dark"
```
