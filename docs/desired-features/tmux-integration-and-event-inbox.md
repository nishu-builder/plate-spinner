# Feature: tmux Integration + Event Inbox

## Problem

Multiple Claude Code sessions run concurrently. External systems (CI, code review, reminders, custom scripts) may want to send work to these sessions, but there's no way to:
1. Inject prompts into running sessions
2. Queue external events for dispatch to sessions

## Solution

Two features working together:

1. **tmux integration** - `sp run` wraps sessions in tmux, enabling programmatic prompt injection via `tmux send-keys`
2. **Event inbox** - Generic event queue with manual dispatch to sessions

---

## Part 1: tmux Integration

### Behavior

`sp run` will:
1. Create tmux session with name `sp-{7-char-hash}`
2. Run `claude` inside it
3. Attach user to it

Benefits:
- Inject prompts: `tmux send-keys -t sp-abc123 "Fix the thing" Enter`
- Resume: `tmux attach -t sp-abc123`
- Detach: `ctrl-b d`

### Edge Cases

- **Already in tmux**: Create window instead of nested session
- **tmux not installed**: Error with clear message (tmux is required)

### Schema Change

```sql
ALTER TABLE sessions ADD COLUMN tmux_target TEXT;
```

---

## Part 2: Event Inbox

### Architecture: Two Separate Event Systems

plate-spinner has two distinct event flows:

| Aspect | CC Hook Events (`/event`) | Inbox Events (`/inbox`) |
|--------|---------------------------|-------------------------|
| Source | Internal (Claude Code sessions) | External (anything) |
| Processing | Immediate, automatic | Queued, manual dispatch |
| Purpose | Session state tracking | Action requests |
| Tied to session | Always (has session_id) | Not necessarily |
| Schema | Specific (tool_name, params) | Generic (type, title, body) |

These remain separate. CC hooks update session state automatically. Inbox events queue for human triage.

### Generic Event Schema

```json
{
  "type": "string",      // required - user-defined, e.g. "review", "ci", "reminder"
  "title": "string",     // required - one-line summary
  "body": "string",      // optional - full content
  "context": {}          // optional - arbitrary k/v metadata for templates
}
```

The schema is intentionally minimal and generic. Users define their own `type` values and configure templates per type.

### Database

```sql
CREATE TABLE inbox (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    type TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT,
    context TEXT,            -- JSON blob
    status TEXT DEFAULT 'pending',  -- pending/dispatched/dismissed/snoozed
    snoozed_until TEXT,
    dispatched_to TEXT,      -- session_id
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### API

- `POST /inbox` - Create event
- `GET /inbox` - List events (optional `?status=pending`)
- `PATCH /inbox/{id}` - Update status
- `POST /inbox/{id}/dispatch` - Send to session via tmux

### Example Usage

Anyone can POST events:

```bash
# GitHub webhook transformer
curl -X POST localhost:7890/inbox -d '{
  "type": "review",
  "title": "Changes requested on #42",
  "body": "Reviewer says: please add tests",
  "context": {"pr": 42, "repo": "owner/repo", "url": "https://..."}
}'

# CI failure notification
curl -X POST localhost:7890/inbox -d '{
  "type": "ci",
  "title": "Build failed: lint",
  "body": "Error: unused import on line 42",
  "context": {"job": "lint", "branch": "feature-x"}
}'

# Personal reminder
curl -X POST localhost:7890/inbox -d '{
  "type": "reminder",
  "title": "Check on the deploy"
}'

# Pipe from anywhere
echo '{"type":"note","title":"Look at this"}' | curl -X POST localhost:7890/inbox -d @-
```

### User-Configurable Templates

Templates live in config, keyed by `type`:

```yaml
inbox:
  templates:
    review: |
      Code review feedback:

      {title}

      {body}

      PR: {context.url}

      Please address this feedback.

    ci: |
      CI failed: {title}

      {body}

      Please investigate and fix.

    default: |
      [{type}] {title}

      {body}
```

Templates can reference `{type}`, `{title}`, `{body}`, and any `{context.KEY}` field.

### Dashboard UX

Layout: Two sections - SESSIONS and INBOX

Keybindings:
| Key | Action |
|-----|--------|
| `tab` | Switch focus between sessions/inbox |
| `d` | Quick dispatch - render template, send immediately |
| `D` | Edit dispatch - open editor to customize prompt before sending |
| `n` | New session with event context |
| `x` | Dismiss event |
| `z` | Snooze event |

### Dispatch Mechanism

When dispatching event to session:
1. Look up session's `tmux_target`
2. Render template for event's `type`
3. `tmux send-keys -t {tmux_target} "{rendered_prompt}" Enter`
4. Mark event as `dispatched`, record `dispatched_to`

---

## Integration Examples

### GitHub Webhooks

Small transformer script that receives GitHub webhook, extracts relevant info, POSTs to inbox:

```bash
# In your webhook receiver
jq -n \
  --arg type "review" \
  --arg title "$PR_TITLE" \
  --arg body "$COMMENT_BODY" \
  --arg url "$PR_URL" \
  '{type: $type, title: $title, body: $body, context: {url: $url}}' \
| curl -X POST localhost:7890/inbox -d @-
```

### CI Systems

Add a failure hook that POSTs to inbox:

```yaml
# .github/workflows/ci.yml
- name: Notify plate-spinner on failure
  if: failure()
  run: |
    curl -X POST localhost:7890/inbox -d "{
      \"type\": \"ci\",
      \"title\": \"${{ github.job }} failed\",
      \"body\": \"See logs at ${{ github.server_url }}/${{ github.repository }}/actions/runs/${{ github.run_id }}\"
    }"
```

### Custom Scripts

```bash
# notify-plate-spinner.sh - generic helper
#!/bin/bash
curl -X POST localhost:7890/inbox \
  -H "Content-Type: application/json" \
  -d "{\"type\": \"$1\", \"title\": \"$2\", \"body\": \"$3\"}"

# Usage
notify-plate-spinner "reminder" "Review the PR" "It's been open for 2 days"
```

---

## Out of Scope (for now)

- Auto-dispatch based on matching rules
- Webhook receiver with auth/verification (user runs their own transformer)
- Bidirectional communication (inbox is fire-and-forget)
- Event deduplication
