use anyhow::Result;
use std::path::Path;

use crate::config::{get_config_path, load_config, save_config, Config};

pub fn config_path() -> Result<()> {
    println!("{}", get_config_path().display());
    Ok(())
}

pub fn config_export() -> Result<()> {
    let path = get_config_path();
    if path.exists() {
        let contents = std::fs::read_to_string(&path)?;
        print!("{}", contents);
    } else {
        let config = load_config();
        save_config(&config)?;
        let contents = std::fs::read_to_string(&path)?;
        print!("{}", contents);
    }
    Ok(())
}

pub fn config_import(file: &str) -> Result<()> {
    let contents = std::fs::read_to_string(Path::new(file))?;
    let config: Config = toml::from_str(&contents)?;
    save_config(&config)?;
    println!("Imported config from {}", file);
    Ok(())
}

pub fn config_set(key: &str, value: &str) -> Result<()> {
    let mut config = load_config();

    match key {
        "tmux_mode" => {
            config.tmux_mode = match value.to_lowercase().as_str() {
                "true" | "1" | "on" | "yes" => true,
                "false" | "0" | "off" | "no" => false,
                _ => anyhow::bail!("Invalid value for tmux_mode: use true/false"),
            };
        }
        "sounds.enabled" => {
            config.sounds.enabled = match value.to_lowercase().as_str() {
                "true" | "1" | "on" | "yes" => true,
                "false" | "0" | "off" | "no" => false,
                _ => anyhow::bail!("Invalid value for sounds.enabled: use true/false"),
            };
        }
        _ => anyhow::bail!(
            "Unknown config key: {}\nAvailable keys: tmux_mode, sounds.enabled",
            key
        ),
    }

    save_config(&config)?;
    println!("{} = {}", key, value);
    Ok(())
}
