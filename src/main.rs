use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sp", about = "Dashboard for managing Claude Code sessions")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Daemon,
    Run {
        #[arg(trailing_var_arg = true)]
        claude_args: Vec<String>,
    },
    Sessions,
    Install,
    Kill,
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommands>,
    },
    Hook {
        #[command(subcommand)]
        hook_type: HookCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    Path,
    Export,
    Import { file: String },
}

#[derive(Subcommand)]
enum HookCommands {
    SessionStart,
    PreToolUse,
    PostToolUse,
    Stop,
}

fn main() {
    let cli = Cli::parse();
    println!("plate-spinner stub");
}
