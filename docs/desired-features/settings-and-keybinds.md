# Settings Menu and Keybind Customization

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
pin = "p"
settings = "s"
# etc
```

## Settings Menu Enhancements

Current settings menu (`s`) supports themes and sounds. Future additions:

- Change keybinds
- Set Anthropic API key
- Export/import config

### Open Questions

- Should settings be a modal overlay or a full-screen view?
- How to handle keybind conflicts?
