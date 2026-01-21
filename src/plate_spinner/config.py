import importlib.resources
from pathlib import Path
from typing import Literal

import tomli_w
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
    except (tomllib.TOMLDecodeError, ValueError, OSError):
        return Config()


def save_config(config: Config) -> None:
    path = get_config_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(tomli_w.dumps(config.model_dump()))


def get_sound_path(name: str) -> Path | None:
    if name == "none":
        return None
    try:
        ref = importlib.resources.files("plate_spinner.sounds") / f"{name}.wav"
        with importlib.resources.as_file(ref) as path:
            return Path(path) if path.exists() else None
    except OSError:
        return None


def get_available_sounds() -> list[str]:
    return ["chime", "bell", "pop", "ping", "alert", "none"]
