use anyhow::Result;
use std::path::PathBuf;

fn claude_settings_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
        .join("settings.json")
}

pub fn hooks_installed() -> bool {
    let path = claude_settings_path();
    if !path.exists() {
        return false;
    }
    match std::fs::read_to_string(&path) {
        Ok(contents) => contents.contains("sp hook"),
        Err(_) => false,
    }
}

pub fn warn_if_hooks_missing() {
    if !hooks_installed() {
        eprintln!("Warning: hooks not installed. Run `sp install` for setup instructions.");
        eprintln!();
    }
}

pub fn install() -> Result<()> {
    let hooks_json = serde_json::json!({
        "hooks": {
            "SessionStart": [{
                "hooks": [{
                    "type": "command",
                    "command": "[ \"$PLATE_SPINNER\" = \"1\" ] && sp hook session-start || true"
                }]
            }],
            "UserPromptSubmit": [{
                "hooks": [{
                    "type": "command",
                    "command": "[ \"$PLATE_SPINNER\" = \"1\" ] && sp hook prompt-submit || true"
                }]
            }],
            "PreToolUse": [{
                "matcher": "*",
                "hooks": [{
                    "type": "command",
                    "command": "[ \"$PLATE_SPINNER\" = \"1\" ] && sp hook pre-tool-use || true"
                }]
            }],
            "PostToolUse": [{
                "matcher": "*",
                "hooks": [{
                    "type": "command",
                    "command": "[ \"$PLATE_SPINNER\" = \"1\" ] && sp hook post-tool-use || true"
                }]
            }],
            "Stop": [{
                "hooks": [{
                    "type": "command",
                    "command": "[ \"$PLATE_SPINNER\" = \"1\" ] && sp hook stop || true"
                }]
            }]
        }
    });

    println!("Add to ~/.claude/settings.json:");
    println!("{}", serde_json::to_string_pretty(&hooks_json)?);

    Ok(())
}
