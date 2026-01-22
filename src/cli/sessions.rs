use anyhow::Result;

use crate::hook::DAEMON_URL;

pub fn sessions() -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(format!("{}/sessions", DAEMON_URL))
        .timeout(std::time::Duration::from_secs(5))
        .send()?;

    let json: serde_json::Value = response.json()?;
    println!("{}", serde_json::to_string_pretty(&json)?);

    Ok(())
}
