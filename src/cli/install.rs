use anyhow::Result;

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
