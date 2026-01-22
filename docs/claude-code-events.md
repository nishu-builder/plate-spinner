# Claude Code Events and State Machine

This document describes how plate-spinner receives events from Claude Code and manages session state.

## Claude Code Hooks

Claude Code provides hooks that fire at specific points in a session's lifecycle. We register handlers for these hooks in `~/.claude/settings.json`:

| Hook | When it fires | Our handler |
|------|---------------|-------------|
| `SessionStart` | When a new Claude Code session begins | `sp hook session-start` |
| `UserPromptSubmit` | When the user submits a message | `sp hook prompt-submit` |
| `PreToolUse` | Before Claude executes a tool | `sp hook pre-tool-use` |
| `PostToolUse` | After Claude finishes executing a tool | `sp hook post-tool-use` |
| `Stop` | When the session ends (exit, error, or timeout) | `sp hook stop` |

### Hook Data

Each hook receives JSON on stdin from Claude Code. The data varies by hook:

**SessionStart:**
```json
{
  "session_id": "uuid",
  "cwd": "/path/to/project",
  "transcript_path": "/path/to/transcript.jsonl"
}
```

**UserPromptSubmit:**
```json
{
  "session_id": "uuid",
  "cwd": "/path/to/project"
}
```

**PreToolUse / PostToolUse:**
```json
{
  "session_id": "uuid",
  "cwd": "/path/to/project",
  "tool_name": "Bash",
  "tool_input": { ... }
}
```

**Stop:**
```json
{
  "session_id": "uuid",
  "cwd": "/path/to/project",
  "error": null | "error message"
}
```

## Events We Send to Daemon

Our hooks translate Claude Code hook data into events posted to the daemon at `POST /events`:

| Claude Hook | Event Type | Notes |
|-------------|------------|-------|
| SessionStart | `session_start` | Includes transcript_path and git_branch |
| UserPromptSubmit | `prompt_submit` | |
| PreToolUse | `tool_start` | Includes tool_name and tool_input |
| PostToolUse | `tool_call` | Includes tool_name |
| Stop | `stop` | Includes error if present |

## State Machine

### PlateStatus Enum

```
Starting    - Placeholder registered, waiting for session_start
Running     - Claude is actively working
Idle        - Session stopped normally, waiting for user
AwaitingInput    - Claude called AskUserQuestion, waiting for user response
AwaitingApproval - Claude called ExitPlanMode, waiting for plan approval
Error       - Session stopped with an error
Closed      - Session terminated
```

### Status Transitions

```
                                    ┌─────────────────────────────────────┐
                                    │                                     │
                                    ▼                                     │
┌──────────┐  session_start   ┌─────────┐  prompt_submit    ┌─────────┐  │
│ Starting │ ───────────────► │ Running │ ◄──────────────── │  Idle   │  │
└──────────┘                  └─────────┘                   └─────────┘  │
                                   │                             ▲       │
                                   │ tool_start                  │       │
                                   │                             │       │
                    ┌──────────────┼──────────────┐              │       │
                    │              │              │              │       │
                    ▼              ▼              ▼              │       │
             ┌───────────┐  ┌───────────┐  ┌───────────┐        │       │
             │ Awaiting  │  │ Awaiting  │  │  Running  │        │       │
             │   Input   │  │  Approval │  │ (default) │        │       │
             └───────────┘  └───────────┘  └───────────┘        │       │
                    │              │              │              │       │
                    │              │              │              │       │
                    │   tool_call  │              │              │       │
                    └──────────────┼──────────────┘              │       │
                                   │                             │       │
                                   ▼                             │       │
                              ┌─────────┐                        │       │
                              │ Running │ ───────────────────────┘       │
                              └─────────┘     stop (no error)            │
                                   │                                     │
                                   │ stop (with error)                   │
                                   ▼                                     │
                              ┌─────────┐                                │
                              │  Error  │                                │
                              └─────────┘                                │
                                                                         │
                              ┌─────────┐                                │
                              │ Closed  │ ◄──────────────────────────────┘
                              └─────────┘   mark_stopped (external)
```

### Transition Rules (determine_status)

```rust
match event.event_type {
    "stop" => if error { Error } else { Idle },
    "prompt_submit" => Running,
    "session_start" => Running,
    "tool_start" => match tool_name {
        "AskUserQuestion" => AwaitingInput,
        "ExitPlanMode" => AwaitingApproval,
        _ => Running,
    },
    "tool_call" => Running,
    _ => Running,
}
```

## Tool-Specific Status Mapping

| Tool | Status on tool_start |
|------|---------------------|
| `AskUserQuestion` | `AwaitingInput` |
| `ExitPlanMode` | `AwaitingApproval` |
| All other tools | `Running` |

On `tool_call` (tool completion), status always returns to `Running`.

## Known Issues

### 1. ExitPlanMode PostToolUse hook doesn't fire

**Symptom:** Session shows `AwaitingApproval` but is actually idle at input prompt.

**Cause:** Claude Code does not fire `PostToolUse` hooks for `ExitPlanMode`. We receive `tool_start` but never `tool_call`.

**Evidence:**
```sql
SELECT event_type, COUNT(*) FROM events
WHERE json_extract(payload, '$.tool_name') = 'ExitPlanMode'
GROUP BY event_type;
-- Result: tool_start|2, tool_call|0
```

**Impact:**
- If Claude uses another tool after plan approval, status recovers via that tool's `tool_start`
- If Claude goes directly to idle (text output only), status stays stuck at `AwaitingApproval`

**Workaround needed:** Health check that compares transcript mtime to last event time.

### 2. AskUserQuestion works correctly

**Verified:** `AskUserQuestion` does fire `PostToolUse` hooks:
```sql
SELECT event_type, COUNT(*) FROM events
WHERE json_extract(payload, '$.tool_name') = 'AskUserQuestion'
GROUP BY event_type;
-- Result: tool_start|4, tool_call|3
```

The missing `tool_call` (3 vs 4) is likely from a session that was interrupted before the user responded.

### 3. No "turn complete" event

**Gap:** There's no Claude Code hook for "Claude finished responding and is waiting for input." The `Stop` hook only fires when the session fully terminates.

**Impact:** After Claude finishes a turn (text output, no more tools), we don't receive any event. Status remains whatever it was during the last tool call.

**Example flow:**
1. `tool_start|ExitPlanMode` → AwaitingApproval
2. User approves plan
3. (No tool_call event - bug #1)
4. Claude prints summary text
5. Claude waits for input
6. (No event - gap #3)
7. Status stuck at AwaitingApproval

### 4. Race conditions with rapid events

**Scenario:** Multiple tools executing rapidly could have events processed out of order.

**Mitigation:** Events include timestamps, but we don't currently enforce ordering.

### 5. Silent hook failures

**Issue:** Hooks use `|| true` to avoid blocking Claude Code, but this masks failures.

**Impact:** If daemon is unreachable during a specific event, we miss it with no indication.

## Proposed Fix: Transcript Health Check

To address issues #1-3, implement periodic validation:

1. For sessions in "attention-needed" states (AwaitingInput, AwaitingApproval), check transcript mtime
2. If transcript was modified after our last event, the session advanced without us knowing
3. Reset status to Idle (or Running if we can detect activity)

This is a generic solution that catches any missed events, not just ExitPlanMode.
