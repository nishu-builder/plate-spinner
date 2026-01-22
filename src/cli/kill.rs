use anyhow::Result;
use std::process::Command;

pub fn kill() -> Result<()> {
    let patterns = ["plate-spinner.*daemon", "sp daemon"];
    let mut killed = false;

    for pattern in patterns {
        let result = Command::new("pkill")
            .args(["-f", pattern])
            .output();

        if let Ok(output) = result {
            if output.status.success() {
                killed = true;
            }
        }
    }

    if killed {
        println!("Daemon stopped");
    } else {
        println!("No daemon running");
    }

    Ok(())
}
