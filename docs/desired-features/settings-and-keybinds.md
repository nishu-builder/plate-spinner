# Settings Menu and Keybind Customization

## Theme Toggle

- `c` to cycle through available themes
- Visual feedback when theme changes

## Pin Plates

- `p` to pin a plate in its current position
- Pinned plates don't shift when others are added/removed
- Visual indicator for pinned state

## Configurable Keybinds

Store keybinds in config.toml:

```toml
[keybinds]
quit = "q"
add = "a"
focus = "f"
close = "x"
theme = "c"
pin = "p"
settings = "s"
# etc
```

## Settings Menu (`s`)

Rethink the bottom-bar approach. Current commands are getting crowded. Consider a dedicated settings screen:

- Change keybinds
- Themes
- Sounds
- Set Anthropic API key
- Export/import config

This would replace having every command visible at the bottom. The bottom bar could show just the essentials (quit, add, focus) and mention "s for settings" for everything config-related.

### Open Questions

- Should settings be a modal overlay or a full-screen view?
- How to handle keybind conflicts?
- Should theme toggle (`c`) remain a quick-access key even with settings menu?
