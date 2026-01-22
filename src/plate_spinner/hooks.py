from pathlib import Path

SCRIPTS_DIR = Path(__file__).parent.parent.parent / "plugin" / "scripts"
HOOK_SCRIPT_NAMES = ["session-start.sh", "pre-tool-use.sh", "post-tool-use.sh", "stop.sh"]


def hooks_installed() -> bool:
    hooks_dir = Path.home() / ".plate-spinner" / "hooks"
    if not hooks_dir.exists():
        return False
    for name in HOOK_SCRIPT_NAMES:
        src = SCRIPTS_DIR / name
        dst = hooks_dir / name
        if not dst.exists():
            return False
        if src.read_text() != dst.read_text():
            return False
    return True
