use serde::{Deserialize, Serialize};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub tmux_mode: bool,
    #[serde(default = "default_true")]
    pub minimal_mode: bool,
    #[serde(default)]
    pub sounds: SoundsConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_tap")]
    pub awaiting_input: String,
    #[serde(default = "default_bell")]
    pub awaiting_approval: String,
    #[serde(default = "default_error")]
    pub error: String,
    #[serde(default = "default_pop")]
    pub idle: String,
    #[serde(default = "default_none")]
    pub closed: String,
}

impl Default for SoundsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            awaiting_input: "tap".to_string(),
            awaiting_approval: "bell".to_string(),
            error: "error".to_string(),
            idle: "pop".to_string(),
            closed: "none".to_string(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_tap() -> String {
    "tap".to_string()
}
fn default_bell() -> String {
    "bell".to_string()
}
fn default_error() -> String {
    "error".to_string()
}
fn default_pop() -> String {
    "pop".to_string()
}
fn default_none() -> String {
    "none".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_theme")]
    pub name: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "textual-dark".to_string(),
        }
    }
}

fn default_theme() -> String {
    "default".to_string()
}

pub const AVAILABLE_THEMES: &[&str] = &["default", "light", "monochrome"];

pub fn get_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".config")
        .join("plate-spinner")
        .join("config.toml")
}

pub fn get_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/share"))
        .join("plate-spinner")
}

pub fn load_config() -> Config {
    let path = get_config_path();
    if !path.exists() {
        return Config::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(config: &Config) -> anyhow::Result<()> {
    let path = get_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, toml::to_string_pretty(config)?)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub anthropic_api_key: String,
}

pub fn get_auth_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".config")
        .join("plate-spinner")
        .join("auth.toml")
}

pub fn load_auth_config() -> Option<AuthConfig> {
    let path = get_auth_config_path();
    if !path.exists() {
        return None;
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
}

pub fn save_auth_config(config: &AuthConfig) -> anyhow::Result<()> {
    let path = get_auth_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config)?;
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)?
        .write_all(content.as_bytes())?;
    Ok(())
}

pub fn delete_auth_config() -> anyhow::Result<()> {
    let path = get_auth_config_path();
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

pub const AVAILABLE_SOUNDS: &[&str] = &[
    "alert",
    "bell",
    "click",
    "error",
    "long-pop",
    "peon-angry-1",
    "peon-angry-2",
    "peon-angry-3",
    "peon-angry-4",
    "peon-death",
    "peon-ready",
    "peon-what-1",
    "peon-what-2",
    "peon-what-3",
    "peon-what-4",
    "peon-warcry",
    "peon-yes-1",
    "peon-yes-2",
    "peon-yes-3",
    "peon-yes-4",
    "peon-yes-attack-1",
    "peon-yes-attack-2",
    "peon-yes-attack-3",
    "pop",
    "tap",
    "none",
];

pub const SOUND_ALERT: &[u8] = include_bytes!("../sounds/alert.wav");
pub const SOUND_BELL: &[u8] = include_bytes!("../sounds/bell.wav");
pub const SOUND_CLICK: &[u8] = include_bytes!("../sounds/click.wav");
pub const SOUND_ERROR: &[u8] = include_bytes!("../sounds/error.wav");
pub const SOUND_LONG_POP: &[u8] = include_bytes!("../sounds/long-pop.wav");
pub const SOUND_POP: &[u8] = include_bytes!("../sounds/pop.wav");
pub const SOUND_TAP: &[u8] = include_bytes!("../sounds/tap.wav");
pub const SOUND_PEON_READY: &[u8] = include_bytes!("../sounds/peon-ready.wav");
pub const SOUND_PEON_WHAT_1: &[u8] = include_bytes!("../sounds/peon-what-1.wav");
pub const SOUND_PEON_WHAT_2: &[u8] = include_bytes!("../sounds/peon-what-2.wav");
pub const SOUND_PEON_WHAT_3: &[u8] = include_bytes!("../sounds/peon-what-3.wav");
pub const SOUND_PEON_WHAT_4: &[u8] = include_bytes!("../sounds/peon-what-4.wav");
pub const SOUND_PEON_YES_1: &[u8] = include_bytes!("../sounds/peon-yes-1.wav");
pub const SOUND_PEON_YES_2: &[u8] = include_bytes!("../sounds/peon-yes-2.wav");
pub const SOUND_PEON_YES_3: &[u8] = include_bytes!("../sounds/peon-yes-3.wav");
pub const SOUND_PEON_YES_4: &[u8] = include_bytes!("../sounds/peon-yes-4.wav");
pub const SOUND_PEON_YES_ATTACK_1: &[u8] = include_bytes!("../sounds/peon-yes-attack-1.wav");
pub const SOUND_PEON_YES_ATTACK_2: &[u8] = include_bytes!("../sounds/peon-yes-attack-2.wav");
pub const SOUND_PEON_YES_ATTACK_3: &[u8] = include_bytes!("../sounds/peon-yes-attack-3.wav");
pub const SOUND_PEON_ANGRY_1: &[u8] = include_bytes!("../sounds/peon-angry-1.wav");
pub const SOUND_PEON_ANGRY_2: &[u8] = include_bytes!("../sounds/peon-angry-2.wav");
pub const SOUND_PEON_ANGRY_3: &[u8] = include_bytes!("../sounds/peon-angry-3.wav");
pub const SOUND_PEON_ANGRY_4: &[u8] = include_bytes!("../sounds/peon-angry-4.wav");
pub const SOUND_PEON_DEATH: &[u8] = include_bytes!("../sounds/peon-death.wav");
pub const SOUND_PEON_WARCRY: &[u8] = include_bytes!("../sounds/peon-warcry.wav");

pub fn get_sound_bytes(name: &str) -> Option<&'static [u8]> {
    match name {
        "alert" => Some(SOUND_ALERT),
        "bell" => Some(SOUND_BELL),
        "click" => Some(SOUND_CLICK),
        "error" => Some(SOUND_ERROR),
        "long-pop" => Some(SOUND_LONG_POP),
        "pop" => Some(SOUND_POP),
        "tap" => Some(SOUND_TAP),
        "peon-angry-1" => Some(SOUND_PEON_ANGRY_1),
        "peon-angry-2" => Some(SOUND_PEON_ANGRY_2),
        "peon-angry-3" => Some(SOUND_PEON_ANGRY_3),
        "peon-angry-4" => Some(SOUND_PEON_ANGRY_4),
        "peon-death" => Some(SOUND_PEON_DEATH),
        "peon-ready" => Some(SOUND_PEON_READY),
        "peon-what-1" => Some(SOUND_PEON_WHAT_1),
        "peon-what-2" => Some(SOUND_PEON_WHAT_2),
        "peon-what-3" => Some(SOUND_PEON_WHAT_3),
        "peon-what-4" => Some(SOUND_PEON_WHAT_4),
        "peon-warcry" => Some(SOUND_PEON_WARCRY),
        "peon-yes-1" => Some(SOUND_PEON_YES_1),
        "peon-yes-2" => Some(SOUND_PEON_YES_2),
        "peon-yes-3" => Some(SOUND_PEON_YES_3),
        "peon-yes-4" => Some(SOUND_PEON_YES_4),
        "peon-yes-attack-1" => Some(SOUND_PEON_YES_ATTACK_1),
        "peon-yes-attack-2" => Some(SOUND_PEON_YES_ATTACK_2),
        "peon-yes-attack-3" => Some(SOUND_PEON_YES_ATTACK_3),
        _ => None,
    }
}

pub fn play_sound(name: &str) {
    let Some(bytes) = get_sound_bytes(name) else {
        return;
    };
    let bytes = bytes.to_vec();
    std::thread::spawn(move || {
        if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
            if let Ok(source) = rodio::Decoder::new(std::io::Cursor::new(bytes)) {
                use rodio::Source;
                let _ = handle.play_raw(source.convert_samples());
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    });
}
