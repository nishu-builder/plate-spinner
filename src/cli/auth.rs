use crate::config::{delete_auth_config, get_auth_config_path, save_auth_config, AuthConfig};
use anyhow::Result;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::{self, Read, Write};

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

    let key = read_masked_input()?;
    println!();

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

fn read_masked_input() -> Result<String> {
    enable_raw_mode()?;
    let result = read_masked_input_inner();
    disable_raw_mode()?;
    result
}

fn read_masked_input_inner() -> Result<String> {
    let mut input = String::new();
    let mut stdin = io::stdin();
    let mut buf = [0u8; 1];

    loop {
        if stdin.read(&mut buf)? == 0 {
            break;
        }
        match buf[0] {
            b'\n' | b'\r' => break,
            127 | 8 => {
                if !input.is_empty() {
                    input.pop();
                }
            }
            3 => anyhow::bail!("Cancelled"),
            c if c >= 32 => {
                input.push(c as char);
            }
            _ => {}
        }
    }
    Ok(input)
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
