# Feature: Push Notifications

## Problem

When plates need attention (awaiting input, awaiting approval, errors), the only way to know is by watching the dashboard. This doesn't scale when multitasking or away from the terminal.

## Solution

Push notifications to devices when plates reach attention-requiring states:

- Apple Watch haptic buzzes
- macOS notification center
- Phone push notifications (iOS/Android)

## Triggers

Notify when plate transitions to:
- `awaiting_input` (AskUserQuestion)
- `awaiting_approval` (ExitPlanMode)
- `error`

## Implementation Options

### macOS Native (simplest)
Use `osascript` or `terminal-notifier` for Notification Center alerts.

### Apple Watch / iOS
Requires a relay service (Pushover, Pushcut, ntfy.sh, custom APNs).

### Cross-platform
ntfy.sh - self-hostable, supports iOS/Android/desktop.

## Configuration

```toml
[notifications]
enabled = true
on_states = ["awaiting_input", "awaiting_approval", "error"]
provider = "ntfy"  # or "macos", "pushover", etc.

[notifications.ntfy]
topic = "plate-spinner"
server = "https://ntfy.sh"  # or self-hosted
```

## Out of Scope (for now)

- Sound customization per event type
- Notification grouping/batching
- Do-not-disturb schedules
