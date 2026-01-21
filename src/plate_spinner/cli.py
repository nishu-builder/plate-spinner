import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

import httpx
import uvicorn

from .daemon.app import create_app
from .daemon.db import Database


def get_db_path() -> Path:
    return Path.home() / ".plate-spinner" / "state.db"


def daemon_running() -> bool:
    try:
        httpx.get("http://localhost:7890/health", timeout=1)
        return True
    except Exception:
        return False


def cmd_daemon(args: argparse.Namespace) -> None:
    db = Database(get_db_path())
    app = create_app(db)
    uvicorn.run(app, host="127.0.0.1", port=7890, log_level="warning")


def cmd_tui(args: argparse.Namespace) -> None:
    if not daemon_running():
        subprocess.Popen(
            [sys.executable, "-m", "plate_spinner.cli", "daemon"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
        time.sleep(1)

    from .tui.app import run
    run()


def cmd_run(args: argparse.Namespace) -> None:
    env = os.environ.copy()
    env["PLATE_SPINNER"] = "1"
    os.execvpe("claude", ["claude"] + args.claude_args, env)


def cmd_sessions(args: argparse.Namespace) -> None:
    try:
        response = httpx.get("http://localhost:7890/sessions", timeout=5)
        print(json.dumps(response.json(), indent=2))
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


def cmd_install(args: argparse.Namespace) -> None:
    import shutil

    hooks_dir = Path.home() / ".plate-spinner" / "hooks"
    hooks_dir.mkdir(parents=True, exist_ok=True)

    pkg_dir = Path(__file__).parent.parent.parent / "plugin" / "scripts"
    for script in ["post-tool-use.sh", "stop.sh"]:
        src = pkg_dir / script
        dst = hooks_dir / script
        if src.exists():
            shutil.copy(src, dst)
            dst.chmod(0o755)
            print(f"Installed {dst}")
        else:
            print(f"Warning: {src} not found", file=sys.stderr)

    print("\nAdd to ~/.claude/settings.json:")
    print(json.dumps({
        "hooks": {
            "PostToolUse": [{
                "matcher": "*",
                "hooks": [{
                    "type": "command",
                    "command": f'[ "$PLATE_SPINNER" = "1" ] && {hooks_dir}/post-tool-use.sh || true'
                }]
            }],
            "Stop": [{
                "hooks": [{
                    "type": "command",
                    "command": f'[ "$PLATE_SPINNER" = "1" ] && {hooks_dir}/stop.sh || true'
                }]
            }]
        }
    }, indent=2))


def main() -> None:
    parser = argparse.ArgumentParser(prog="sp", description="Plate-Spinner")
    subparsers = parser.add_subparsers(dest="command")

    subparsers.add_parser("daemon", help="Run daemon in foreground")
    subparsers.add_parser("tui", help="Launch TUI")
    subparsers.add_parser("sessions", help="List sessions as JSON")
    subparsers.add_parser("install", help="Install hooks to ~/.plate-spinner")

    run_parser = subparsers.add_parser("run", help="Launch Claude with tracking")
    run_parser.add_argument("claude_args", nargs="*", default=[])

    args = parser.parse_args()

    if args.command == "daemon":
        cmd_daemon(args)
    elif args.command == "run":
        cmd_run(args)
    elif args.command == "sessions":
        cmd_sessions(args)
    elif args.command == "install":
        cmd_install(args)
    else:
        cmd_tui(args)


if __name__ == "__main__":
    main()
