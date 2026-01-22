use clap::{Parser, Subcommand};
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::sync::Arc;

use plate_spinner::config::get_data_dir;
use plate_spinner::daemon::state::AppState;
use plate_spinner::db::Database;
use plate_spinner::ensure_daemon_running;

#[derive(Parser)]
#[command(name = "sp", about = "Dashboard for managing Claude Code sessions")]
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
    #[command(about = "List sessions as JSON")]
    Sessions,
    #[command(about = "Install hooks")]
    Install,
    #[command(about = "Stop the daemon")]
    Kill,
    #[command(about = "Manage configuration")]
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommands>,
    },
    #[command(about = "Hook handlers (called by Claude Code)")]
    Hook {
        #[command(subcommand)]
        hook_type: HookCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    #[command(about = "Print config file path")]
    Path,
    #[command(about = "Export config to stdout")]
    Export,
    #[command(about = "Import config from file")]
    Import { file: String },
}

#[derive(Subcommand)]
enum HookCommands {
    #[command(name = "session-start")]
    SessionStart,
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
        Some(Commands::Sessions) => {
            if let Err(e) = plate_spinner::cli::sessions::sessions() {
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
        Some(Commands::Hook { hook_type }) => {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            rt.block_on(async {
                let result = match hook_type {
                    HookCommands::SessionStart => plate_spinner::hook::session_start().await,
                    HookCommands::PreToolUse => plate_spinner::hook::pre_tool_use().await,
                    HookCommands::PostToolUse => plate_spinner::hook::post_tool_use().await,
                    HookCommands::Stop => plate_spinner::hook::stop().await,
                };
                if let Err(e) = result {
                    eprintln!("Hook error: {}", e);
                }
            });
        }
        None => {
            ensure_daemon_running();
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            let resume = rt.block_on(async { plate_spinner::tui::run().await });

            match resume {
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
