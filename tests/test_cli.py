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


def test_config_export_command():
    result = subprocess.run(
        [sys.executable, "-m", "plate_spinner.cli", "config", "export"],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0
    assert "[sounds]" in result.stdout
    assert "enabled = true" in result.stdout
