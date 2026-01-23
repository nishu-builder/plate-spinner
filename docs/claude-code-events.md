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

The state machine is implemented in `src/state_machine.rs` with type-safe enums and exhaustive pattern matching.

### PlateStatus Enum

```
Starting         - Placeholder registered, waiting for session_start
Running          - Claude is actively working
Idle             - Session stopped normally, waiting for user
AwaitingInput    - Claude called AskUserQuestion, waiting for user response
AwaitingApproval - Claude called ExitPlanMode, waiting for plan approval
Error            - Session stopped with an error
Closed           - Session terminated (set externally, not via state machine)
```

### Event Enum

```rust
pub enum Event {
    SessionStart,
    PromptSubmit,
    ToolStart(Tool),      // Tool: AskUserQuestion | ExitPlanMode | Other
    ToolCall,
    Stop { has_error: bool },
    HealthCheckRecovery,  // Internal event for stale state recovery
}
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
                    │   tool_call / health_check_recovery       │       │
                    └──────────────┼──────────────┘              │       │
                                   │                             │       │
                                   ▼                             │       │
                              ┌─────────┐                        │       │
                              │ Running │ ───────────────────────┘       │
                              └─────────┘     stop (no error)            │
                                   │                                     │
                                   │ stop (with error)                   │
                                   ▼                                     │
                              ┌─────────┐  health_check_recovery         │
                              │  Error  │ ──────────────────────►────────┤
                              └─────────┘                                │
                                                                         │
                              ┌─────────┐                                │
                              │ Closed  │ ◄──────────────────────────────┘
                              └─────────┘   mark_stopped (external)
```

### Transition Rules

See `src/state_machine.rs` for the canonical implementation:

```rust
impl PlateStatus {
    pub fn transition(self, event: &Event) -> PlateStatus {
        match (self, event) {
            (_, Event::SessionStart) => PlateStatus::Running,
            (_, Event::PromptSubmit) => PlateStatus::Running,

            (_, Event::ToolStart(Tool::AskUserQuestion)) => PlateStatus::AwaitingInput,
            (_, Event::ToolStart(Tool::ExitPlanMode)) => PlateStatus::AwaitingApproval,
            (_, Event::ToolStart(Tool::Other)) => PlateStatus::Running,

            (_, Event::ToolCall) => PlateStatus::Running,

            (_, Event::Stop { has_error: true }) => PlateStatus::Error,
            (_, Event::Stop { has_error: false }) => PlateStatus::Idle,

            (PlateStatus::AwaitingInput, Event::HealthCheckRecovery) => PlateStatus::Idle,
            (PlateStatus::AwaitingApproval, Event::HealthCheckRecovery) => PlateStatus::Idle,
            (PlateStatus::Error, Event::HealthCheckRecovery) => PlateStatus::Idle,
            (state, Event::HealthCheckRecovery) => state,
        }
    }
}
```

## Tool-Specific Status Mapping

| Tool | Status on tool_start |
|------|---------------------|
| `AskUserQuestion` | `AwaitingInput` |
| `ExitPlanMode` | `AwaitingApproval` |
| All other tools | `Running` |

On `tool_call` (tool completion), status always returns to `Running`.

## Invariants

The state machine guarantees the following invariants, verified by property-based tests in `src/state_machine.rs`:

### 1. Transition Determinism

For any `(state, event)` pair, `transition()` always produces the same `next_state`.

### 2. State Validity

All transitions land in a valid `PlateStatus` variant. Enforced at compile time by Rust's exhaustive pattern matching.

### 3. Recovery Guarantee

Stuck attention states recover to Idle within bounded time. See proof below.

### 4. Sequence Safety

Any sequence of events maintains a valid state.

## Recovery Guarantee

### Problem

Claude Code hooks have known gaps:
- `ExitPlanMode` does not fire `PostToolUse`
- No "turn complete" event exists
- Hooks can fail silently

These gaps can leave sessions stuck in `AwaitingInput`, `AwaitingApproval`, or `Error` states.

### Solution

A health check runs every 10 seconds, comparing transcript modification time to our last recorded event time. If the transcript advanced without us knowing, we trigger a `HealthCheckRecovery` event.

### Proof of Bounded Recovery

```
THEOREM: Bounded Recovery

Given:
  HEALTH_CHECK_INTERVAL_SECS = 10
  STALENESS_THRESHOLD_SECS = 2
  MAX_RECOVERY_TIME_SECS = 12

  A plate in attention-needed state (AwaitingInput, AwaitingApproval, Error)
  True state has advanced (transcript modified after last recorded event)

Then:
  The plate will recover to Idle within at most 12 seconds.

Proof:
  1. Health check runs every 10 seconds
  2. On each run, it checks: transcript_mtime > last_event_time + 2
  3. If true state advanced, transcript was modified
  4. Maximum wait before detection: 10s (interval) + 2s (threshold) = 12s
  5. Upon detection, plate transitions via HealthCheckRecovery → Idle

QED
```

### Implementation

See `src/recovery.rs` for constants and `src/daemon/health_check.rs` for the health check loop.

```rust
// src/recovery.rs
pub const HEALTH_CHECK_INTERVAL_SECS: u64 = 10;
pub const STALENESS_THRESHOLD_SECS: i64 = 2;
pub const MAX_RECOVERY_TIME_SECS: u64 = 12;

pub fn is_stale(transcript_mtime_secs: i64, last_event_time_secs: i64) -> bool {
    transcript_mtime_secs > last_event_time_secs + STALENESS_THRESHOLD_SECS
}
```

## Two-Mechanism Design

State consistency is maintained by two separate mechanisms that handle different failure modes:

### 1. Health Check Recovery (State Machine)

**Handles:** Missed hook events within a running session.

**Problem:** Claude is still running, but we didn't receive a hook event (e.g., `ExitPlanMode` doesn't fire `PostToolUse`, hooks fail silently).

**Mechanism:** Compare transcript mtime vs last event time. If the transcript advanced but we have no record of it, trigger `HealthCheckRecovery` event.

**Guarantee:** Stuck attention states (AwaitingInput, AwaitingApproval, Error) recover to Idle within 12 seconds. See proof above.

**Scope:** Only covers cases where the transcript advances. If the Claude process dies, the transcript doesn't advance, so this mechanism won't help.

### 2. Process Termination Detection (External)

**Handles:** Claude process exit (normal exit, Ctrl+C, terminal close, signals).

**Problem:** When the Claude process terminates, the Stop hook may not fire (e.g., killed by signal, terminal closed). Even if it does fire, Stop means "turn ended" not "process exited" - the session should be Idle, not Closed.

**Mechanism:** The `sp run` wrapper monitors the subprocess and calls `POST /plates/stopped` when it exits. This directly sets `status = 'closed'` in the database, bypassing the state machine.

**Implementation:**
- Normal exit: `sp run` calls `notify_stopped()` after subprocess returns
- Signal death: Signal handler (SIGHUP, SIGINT, SIGTERM) calls `notify_stopped()` before exiting

**Why separate from state machine:** The state machine models session state within a running process. Process termination is orthogonal - it's not an event from Claude Code, it's the absence of a process. Keeping these separate maintains clarity about what each mechanism guarantees.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        State Consistency                                 │
├─────────────────────────────────┬───────────────────────────────────────┤
│     Health Check Recovery       │    Process Termination Detection      │
├─────────────────────────────────┼───────────────────────────────────────┤
│ Missed hooks within session     │ Process exit                          │
│ Transcript advances, we missed  │ No transcript activity                │
│ HealthCheckRecovery event       │ Direct DB update via mark_stopped     │
│ → Idle                          │ → Closed                              │
│ 12 second guarantee             │ Immediate on process exit             │
└─────────────────────────────────┴───────────────────────────────────────┘
```

## Known Limitations

### 1. ExitPlanMode PostToolUse hook doesn't fire

**Symptom:** Session shows `AwaitingApproval` but is actually idle at input prompt.

**Cause:** Claude Code does not fire `PostToolUse` hooks for `ExitPlanMode`.

**Mitigation:** Health check recovers stuck sessions within 12 seconds.

### 2. No "turn complete" event

**Gap:** There's no Claude Code hook for "Claude finished responding and is waiting for input."

**Mitigation:** Health check detects transcript advancement and recovers.

### 3. Race conditions with rapid events

**Scenario:** Multiple tools executing rapidly could have events processed out of order.

**Mitigation:** Events include timestamps, but we don't currently enforce ordering.

### 4. Silent hook failures

**Issue:** Hooks use `|| true` to avoid blocking Claude Code, but this masks failures.

**Mitigation:** Health check recovers from any missed events within 12 seconds.

### 5. Process termination detection requires `sp run` wrapper

**Issue:** Process termination detection only works when Claude is started via `sp run`. Sessions started directly with `claude` won't transition to Closed when the process exits.

**Mitigation:** Users should use `sp run` to start sessions. Sessions started without it will remain in their last state until manually deleted.
