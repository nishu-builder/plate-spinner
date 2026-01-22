use crate::config::{delete_auth_config, get_auth_config_path, save_auth_config, AuthConfig};
use anyhow::Result;
use std::io::{self, BufRead, Write};

pub fn auth_status() -> Result<()> {
    let path = get_auth_config_path();
    if path.exists() {
        println!("API key configured at {}", path.display());
    } else if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        println!("Using ANTHROPIC_API_KEY from environment");
    } else {
        println!("No API key configured");
        println!("Run `sp auth set` to configure");
    }
    Ok(())
}

pub fn auth_set() -> Result<()> {
    print!("Enter your Anthropic API key: ");
    io::stdout().flush()?;

    let stdin = io::stdin();
    let key = stdin.lock().lines().next().unwrap_or(Ok(String::new()))?;
    let key = key.trim().to_string();

    if key.is_empty() {
        anyhow::bail!("API key cannot be empty");
    }

    let config = AuthConfig {
        anthropic_api_key: key,
    };
    save_auth_config(&config)?;

    println!("API key saved to {}", get_auth_config_path().display());
    Ok(())
}

pub fn auth_unset() -> Result<()> {
    delete_auth_config()?;
    println!("API key removed");
    Ok(())
}

pub fn auth_path() -> Result<()> {
    println!("{}", get_auth_config_path().display());
    Ok(())
}
