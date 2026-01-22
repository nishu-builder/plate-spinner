# Feature: Schema Migrations System

## Problem

As plate-spinner evolves, the SQLite schema needs to change. Currently using idempotent column checks (e.g., "add column if not exists"), which works but doesn't scale well:
- No ordering guarantees between migrations
- No rollback capability
- No record of what's been applied
- Gets messy with many changes

## Shape of Solution

A simple version-based system:

1. **Migrations table** - Tracks applied migrations
   ```sql
   CREATE TABLE schema_migrations (
       version INTEGER PRIMARY KEY,
       applied_at TEXT NOT NULL
   );
   ```

2. **Numbered migration functions** - Each migration has a version number
   ```rust
   fn migrate_001_add_tmux_target(conn: &Connection) -> Result<()>;
   fn migrate_002_add_inbox_table(conn: &Connection) -> Result<()>;
   ```

3. **On startup** - Run any migrations with version > current max

Keep it in-code (no separate SQL files) since this is a single-binary tool.

## When to Build

When we have 3+ schema changes and the idempotent checks feel unwieldy.
