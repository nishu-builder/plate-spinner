from plate_spinner.config import Config, SoundsConfig, load_config, save_config, get_sound_path


def test_get_sound_path():
    path = get_sound_path("chime")
    assert path is not None
    assert path.name == "chime.wav"
    assert path.exists()


def test_get_sound_path_none_returns_none():
    path = get_sound_path("none")
    assert path is None


def test_config_defaults():
    config = Config()
    assert config.sounds.enabled is True
    assert config.sounds.awaiting_input == "chime"
    assert config.sounds.awaiting_approval == "bell"
    assert config.sounds.error == "alert"
    assert config.sounds.idle == "pop"
    assert config.sounds.closed == "none"
    assert config.theme.name == "textual-dark"


def test_config_save_and_load(tmp_path, monkeypatch):
    config_path = tmp_path / "config.toml"
    monkeypatch.setattr("plate_spinner.config.get_config_path", lambda: config_path)

    config = Config(sounds=SoundsConfig(awaiting_input="ping", enabled=False))
    save_config(config)

    loaded = load_config()
    assert loaded.sounds.awaiting_input == "ping"
    assert loaded.sounds.enabled is False
