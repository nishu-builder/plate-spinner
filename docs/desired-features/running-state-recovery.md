# Running State Recovery

## Problem

Plates can get stuck in "Running" state when:
1. Stop hook doesn't fire (user interrupt, hook failure, process killed)
2. Session started without `sp run` wrapper (no process termination detection)

Current health check only recovers "attention" states (AwaitingInput, AwaitingApproval, Error), not Running.

## Solution: Transcript-Based Staleness Detection

Key insight: Claude Code writes to the transcript continuously during activity:
- `bash_progress` entries every second during command execution
- `assistant` entries while streaming responses
- `hook_progress` entries during tool execution

**If the transcript hasn't been modified in 30 seconds and status is Running, the session is idle.**

### Implementation

1. **`recovery.rs`**: Add `RUNNING_STALENESS_THRESHOLD_SECS = 30`

2. **`state_machine.rs`**: Add transition
   ```rust
   (PlateStatus::Running, Event::HealthCheckRecovery) => PlateStatus::Idle,
   ```

3. **`health_check.rs`**: For Running plates, check if `now - transcript_mtime > 30s`
   - Different from attention states which check `transcript_mtime > our_last_event_time`

4. **`docs/claude-code-events.md`**: Update recovery guarantee section

### Laptop Sleep/Wake Handling

When system sleeps, transcript mtime becomes stale but session may still be active on wake.

Options:
1. **Track health check cadence**: If time since last health check >> interval, system was asleep. Give 10s grace period before recovering.
2. **Check system wake time**: `sysctl kern.waketime` on macOS
3. **Age-based heuristic**: If transcript age >> threshold (e.g., 5+ minutes), assume sleep, wait before recovering

Recommended: Option 1 - track `last_health_check_time` and skip recovery if gap indicates sleep.

## Design Consideration: Transcript-First Architecture

Now that we know transcript is written to frequently, we could simplify state detection:

| Current Approach | Transcript-First |
|------------------|------------------|
| Hooks → events → state machine transitions | Parse transcript tail → infer state |
| Health check compares mtime to our event time | Health check just checks mtime freshness |
| Multiple mechanisms for different edge cases | Single source of truth |

**Transcript-first benefits:**
- Transcript is ground truth
- No dependency on hook reliability
- Simpler mental model

**Transcript-first costs:**
- More I/O (reading files vs receiving events)
- Parsing complexity
- Coupled to transcript format (could change)

**Current assessment:** The current hook-based system isn't fundamentally flawed. Hooks provide real-time, low-latency updates. The gap is only for Running plates when Stop hook fails. The proposed fix is additive and minimal. A full rewrite isn't warranted unless we see more edge cases.

## Testing

1. Start a session, let it go idle, verify it transitions to Idle within ~30s
2. Start a session, run a 2-minute bash command, verify it stays Running during execution
3. Close laptop with active session, reopen, verify no false positive recovery
4. Kill Claude process without `sp run`, verify plate eventually recovers to Idle
