import tempfile
from pathlib import Path

import pytest

from plate_spinner.daemon.db import Database


def test_database_creates_tables():
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        db = Database(db_path)

        tables = db.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall()
        table_names = {t[0] for t in tables}

        assert "sessions" in table_names
        assert "todos" in table_names
        assert "events" in table_names
