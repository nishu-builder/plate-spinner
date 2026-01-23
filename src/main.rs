use clap::{Parser, Subcommand};
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::sync::Arc;

use plate_spinner::config::get_data_dir;
use plate_spinner::daemon::state::AppState;
use plate_spinner::db::Database;
use plate_spinner::ensure_daemon_running;

#[derive(Parser)]
#[command(name = "sp", about = "Dashboard for managing Claude Code plates")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Run daemon in foreground")]
    Daemon,
    #[command(about = "Launch Claude with tracking")]
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        claude_args: Vec<String>,
    },
    #[command(about = "List plates as JSON")]
    Plates,
    #[command(about = "Install hooks")]
    Install,
    #[command(about = "Stop the daemon")]
    Kill,
    #[command(about = "Manage configuration")]
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommands>,
    },
    #[command(about = "Manage API key authentication")]
    Auth {
        #[command(subcommand)]
        command: Option<AuthCommands>,
    },
    #[command(about = "Hook handlers (called by Claude Code)")]
    Hook {
        #[command(subcommand)]
        hook_type: HookCommands,
    },
    #[command(about = "Run TUI directly (internal)", hide = true)]
    Tui,
}

#[derive(Subcommand)]
enum ConfigCommands {
    #[command(about = "Print config file path")]
    Path,
    #[command(about = "Set a config value")]
    Set { key: String, value: String },
    #[command(about = "Export config to stdout")]
    Export,
    #[command(about = "Import config from file")]
    Import { file: String },
}

#[derive(Subcommand)]
enum AuthCommands {
    #[command(about = "Set API key")]
    Set,
    #[command(about = "Remove stored API key")]
    Unset,
    #[command(about = "Print auth config path")]
    Path,
}

#[derive(Subcommand)]
enum HookCommands {
    #[command(name = "session-start")]
    SessionStart,
    #[command(name = "prompt-submit")]
    PromptSubmit,
    #[command(name = "pre-tool-use")]
    PreToolUse,
    #[command(name = "post-tool-use")]
    PostToolUse,
    Stop,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Daemon) => {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            rt.block_on(async {
                let db_path = get_data_dir().join("state.db");
                let db = Database::open(&db_path).expect("Failed to open database");
                let state = Arc::new(AppState::new(db));
                if let Err(e) = plate_spinner::daemon::run(state, 7890).await {
                    eprintln!("Daemon error: {}", e);
                }
            });
        }
        Some(Commands::Run { claude_args }) => {
            if let Err(e) = plate_spinner::cli::run::run(claude_args) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Plates) => {
            if let Err(e) = plate_spinner::cli::plates::plates() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Install) => {
            if let Err(e) = plate_spinner::cli::install::install() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Kill) => {
            if let Err(e) = plate_spinner::cli::kill::kill() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Config { command }) => {
            let result = match command {
                Some(ConfigCommands::Path) => plate_spinner::cli::config::config_path(),
                Some(ConfigCommands::Set { key, value }) => {
                    plate_spinner::cli::config::config_set(&key, &value)
                }
                Some(ConfigCommands::Export) => plate_spinner::cli::config::config_export(),
                Some(ConfigCommands::Import { file }) => {
                    plate_spinner::cli::config::config_import(&file)
                }
                None => plate_spinner::cli::config::config_path(),
            };
            if let Err(e) = result {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Auth { command }) => {
            let result = match command {
                Some(AuthCommands::Set) => plate_spinner::cli::auth::auth_set(),
                Some(AuthCommands::Unset) => plate_spinner::cli::auth::auth_unset(),
                Some(AuthCommands::Path) => plate_spinner::cli::auth::auth_path(),
                None => plate_spinner::cli::auth::auth_status(),
            };
            if let Err(e) = result {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Hook { hook_type }) => {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            rt.block_on(async {
                let result = match hook_type {
                    HookCommands::SessionStart => plate_spinner::hook::session_start().await,
                    HookCommands::PromptSubmit => plate_spinner::hook::prompt_submit().await,
                    HookCommands::PreToolUse => plate_spinner::hook::pre_tool_use().await,
                    HookCommands::PostToolUse => plate_spinner::hook::post_tool_use().await,
                    HookCommands::Stop => plate_spinner::hook::stop().await,
                };
                if let Err(e) = result {
                    eprintln!("Hook error: {}", e);
                }
            });
        }
        Some(Commands::Tui) => {
            ensure_daemon_running();
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            match rt.block_on(plate_spinner::tui::run()) {
                Ok(Some((session_id, project_path))) => {
                    if let Err(e) = std::env::set_current_dir(&project_path) {
                        eprintln!("Failed to change directory: {}", e);
                        std::process::exit(1);
                    }
                    let err = Command::new("claude")
                        .arg("--resume")
                        .arg(&session_id)
                        .exec();
                    eprintln!("Failed to exec claude: {}", err);
                    std::process::exit(1);
                }
                Ok(None) => {}
                Err(e) => {
                    eprintln!("TUI error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            use plate_spinner::config::load_config;

            let config = load_config();

            if config.tmux_mode {
                use plate_spinner::cli::tmux;

                if let Err(e) = tmux::check_tmux_available() {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
                if let Err(e) = tmux::check_tmux_version() {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }

                let in_tmux = tmux::is_inside_tmux();
                let session = tmux::get_session_name();

                if !in_tmux {
                    if let Err(e) = tmux::ensure_session_exists(&session) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }

                let dashboard_exists = tmux::window_exists(&session, "dashboard");

                if !dashboard_exists {
                    let exe = std::env::current_exe().unwrap_or_else(|_| "sp".into());
                    let exe_str = exe.to_string_lossy();

                    let mut cmd = Command::new("tmux");
                    cmd.args(["new-window", "-n", "dashboard"]);

                    if !in_tmux {
                        cmd.args(["-t", &format!("{}:", &session)]);
                    }

                    cmd.args(["--", &*exe_str, "tui"]);

                    let status = cmd.status().expect("Failed to run tmux");
                    if !status.success() {
                        eprintln!("Failed to create tmux window");
                        std::process::exit(1);
                    }
                }

                if in_tmux {
                    if let Err(e) = tmux::select_window(&format!("{}:dashboard", session)) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                } else {
                    let grouped = tmux::generate_grouped_session_name();
                    let err = Command::new("tmux")
                        .args([
                            "new-session",
                            "-t",
                            &session,
                            "-s",
                            &grouped,
                            ";",
                            "select-window",
                            "-t",
                            "dashboard",
                        ])
                        .exec();
                    eprintln!("Failed to attach to tmux: {}", err);
                    std::process::exit(1);
                }
            } else {
                ensure_daemon_running();
                let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
                match rt.block_on(plate_spinner::tui::run()) {
                    Ok(Some((session_id, project_path))) => {
                        if let Err(e) = std::env::set_current_dir(&project_path) {
                            eprintln!("Failed to change directory: {}", e);
                            std::process::exit(1);
                        }
                        let err = Command::new("claude")
                            .arg("--resume")
                            .arg(&session_id)
                            .exec();
                        eprintln!("Failed to exec claude: {}", err);
                        std::process::exit(1);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        eprintln!("TUI error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}
