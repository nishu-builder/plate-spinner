use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS plates (
    session_id TEXT PRIMARY KEY,
    project_path TEXT NOT NULL,
    transcript_path TEXT,
    git_branch TEXT,
    status TEXT NOT NULL DEFAULT 'running',
    last_event_type TEXT,
    last_tool TEXT,
    summary TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS todos (
    session_id TEXT PRIMARY KEY REFERENCES plates(session_id),
    todos_json TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_plates_status ON plates(status);
CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id);
"#;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(SCHEMA)?;
        self.migrate()?;
        Ok(())
    }

    fn migrate(&self) -> Result<()> {
        let columns: Vec<String> = self
            .conn
            .prepare("PRAGMA table_info(plates)")?
            .query_map([], |row| row.get(1))?
            .filter_map(|r| r.ok())
            .collect();

        if !columns.contains(&"summary".to_string()) {
            self.conn
                .execute("ALTER TABLE plates ADD COLUMN summary TEXT", [])?;
        }
        if !columns.contains(&"transcript_path".to_string()) {
            self.conn
                .execute("ALTER TABLE plates ADD COLUMN transcript_path TEXT", [])?;
        }
        if !columns.contains(&"git_branch".to_string()) {
            self.conn
                .execute("ALTER TABLE plates ADD COLUMN git_branch TEXT", [])?;
        }
        if !columns.contains(&"tmux_target".to_string()) {
            self.conn
                .execute("ALTER TABLE plates ADD COLUMN tmux_target TEXT", [])?;
        }
        if !columns.contains(&"goal".to_string()) {
            self.conn
                .execute("ALTER TABLE plates ADD COLUMN goal TEXT", [])?;
        }
        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    #[allow(clippy::too_many_arguments)]
    pub fn upsert_plate(
        &self,
        session_id: &str,
        project_path: &str,
        transcript_path: Option<&str>,
        git_branch: Option<&str>,
        tmux_target: Option<&str>,
        status: &str,
        event_type: &str,
        tool_name: Option<&str>,
        now: &str,
    ) -> Result<bool> {
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT session_id FROM plates WHERE session_id = ?",
                [session_id],
                |row| row.get(0),
            )
            .ok();

        if existing.is_none() {
            let placeholder_id = format!("pending:{}", project_path);
            self.conn
                .execute("DELETE FROM plates WHERE session_id = ?", [&placeholder_id])?;
            self.conn.execute(
                "INSERT INTO plates (session_id, project_path, transcript_path, git_branch, tmux_target, status, last_event_type, last_tool, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![session_id, project_path, transcript_path, git_branch, tmux_target, status, event_type, tool_name, now, now],
            )?;
            Ok(false)
        } else {
            self.conn.execute(
                "UPDATE plates SET status = ?, last_event_type = ?, last_tool = COALESCE(?, last_tool), transcript_path = COALESCE(?, transcript_path), git_branch = COALESCE(?, git_branch), tmux_target = COALESCE(?, tmux_target), updated_at = ? WHERE session_id = ?",
                params![status, event_type, tool_name, transcript_path, git_branch, tmux_target, now, session_id],
            )?;
            Ok(true)
        }
    }

    pub fn insert_event(
        &self,
        session_id: &str,
        event_type: &str,
        payload: &str,
        now: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events (session_id, event_type, payload, created_at) VALUES (?, ?, ?, ?)",
            params![session_id, event_type, payload, now],
        )?;
        Ok(())
    }

    pub fn upsert_todos(&self, session_id: &str, todos_json: &str, now: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO todos (session_id, todos_json, updated_at) VALUES (?, ?, ?)",
            params![session_id, todos_json, now],
        )?;
        Ok(())
    }

    pub fn get_plates(&self) -> Result<Vec<crate::models::Plate>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT s.session_id, s.project_path, s.git_branch, s.status,
                      s.last_event_type, s.last_tool, s.summary, s.created_at, s.updated_at,
                      s.transcript_path, s.tmux_target, t.todos_json
               FROM plates s
               LEFT JOIN todos t ON s.session_id = t.session_id
               ORDER BY s.updated_at DESC"#,
        )?;

        let rows = stmt.query_map([], |row| {
            let todos_json: Option<String> = row.get(11)?;
            let todo_progress = todos_json.and_then(|json| {
                serde_json::from_str::<Vec<serde_json::Value>>(&json)
                    .ok()
                    .map(|todos| {
                        let completed = todos
                            .iter()
                            .filter(|t| {
                                t.get("status").and_then(|s| s.as_str()) == Some("completed")
                            })
                            .count();
                        format!("{}/{}", completed, todos.len())
                    })
            });

            let status_str: String = row.get(3)?;
            let status = status_str.parse().unwrap_or_default();

            Ok(crate::models::Plate {
                session_id: row.get(0)?,
                project_path: row.get(1)?,
                git_branch: row.get(2)?,
                tmux_target: row.get(10)?,
                status,
                last_event_type: row.get(4)?,
                last_tool: row.get(5)?,
                summary: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                transcript_path: row.get(9)?,
                todo_progress,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn get_transcript_path(&self, session_id: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT transcript_path FROM plates WHERE session_id = ?",
                [session_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn get_summary(&self, session_id: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT summary FROM plates WHERE session_id = ?",
                [session_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn set_summary(&self, session_id: &str, summary: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE plates SET summary = ? WHERE session_id = ?",
            params![summary, session_id],
        )?;
        Ok(())
    }

    pub fn get_goal(&self, session_id: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT goal FROM plates WHERE session_id = ?",
                [session_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn set_goal(&self, session_id: &str, goal: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE plates SET goal = ? WHERE session_id = ?",
            params![goal, session_id],
        )?;
        Ok(())
    }

    pub fn get_event_count(&self, session_id: &str) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE session_id = ?",
                [session_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn register_placeholder(&self, project_path: &str, now: &str) -> Result<String> {
        let placeholder_id = format!("pending:{}", project_path);
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT session_id FROM plates WHERE session_id = ?",
                [&placeholder_id],
                |row| row.get(0),
            )
            .ok();

        if existing.is_none() {
            self.conn.execute(
                "INSERT INTO plates (session_id, project_path, status, created_at, updated_at) VALUES (?, ?, 'starting', ?, ?)",
                params![placeholder_id, project_path, now, now],
            )?;
        }
        Ok(placeholder_id)
    }

    pub fn mark_stopped(&self, project_path: &str, now: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id FROM plates WHERE project_path = ? AND status != 'closed'",
        )?;
        let plate_ids: Vec<String> = stmt
            .query_map([project_path], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        for plate_id in &plate_ids {
            self.conn.execute(
                "UPDATE plates SET status = 'closed', updated_at = ? WHERE session_id = ?",
                params![now, plate_id],
            )?;
        }
        Ok(plate_ids)
    }

    pub fn delete_plate(&self, session_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM todos WHERE session_id = ?", [session_id])?;
        self.conn
            .execute("DELETE FROM events WHERE session_id = ?", [session_id])?;
        self.conn
            .execute("DELETE FROM plates WHERE session_id = ?", [session_id])?;
        Ok(())
    }
}
