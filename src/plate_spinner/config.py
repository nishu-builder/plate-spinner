import importlib.resources
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
