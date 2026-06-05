mod cmd;

use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, shells};
use maggraph::{MagGraphConfig, Result};
use tracing_subscriber::EnvFilter;

use cmd::init::InitArgs;
use cmd::query::QueryArgs;
use cmd::scaffold::ScaffoldArgs;
use cmd::sync::SyncArgs;
use cmd::ui::UiArgs;

#[derive(Debug, Parser)]
#[command(
    name = "maggraph",
    about = "In-process Git-backed graph database for AI",
    version
)]
struct Cli {
    /// Path to maggraph.toml
    #[arg(long, global = true, default_value = "maggraph.toml")]
    config: PathBuf,

    /// Increase logging verbosity (-v info, -vv debug, -vvv trace)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize the graph root (and optional Git repo when [sync] is configured)
    Init(InitArgs),
    /// Traverse the graph and print a Markdown report
    Query(QueryArgs),
    /// Git sync operations (requires [sync] in maggraph.toml)
    Sync(SyncArgs),
    /// Generate agent artifacts (MCP server, SKILL.md)
    Scaffold(ScaffoldArgs),
    /// Start the embedded local web dashboard (localhost only)
    Ui(UiArgs),
    /// Emit shell completion script to stdout
    Complete {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: ShellChoice,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum ShellChoice {
    Bash,
    Zsh,
    Fish,
    Elvish,
    PowerShell,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn init_tracing(verbose: u8) {
    let default_level = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    tracing_subscriber::fmt().with_env_filter(filter).init();
}

fn run(cli: Cli) -> Result<()> {
    // `complete` generates a shell completion script from the CLI structure alone;
    // it does not read any graph config, so we handle it before loading maggraph.toml.
    if let Some(Commands::Complete { shell }) = &cli.command {
        let mut cmd = Cli::command();
        let mut stdout = io::stdout();
        match shell {
            ShellChoice::Bash => generate(shells::Bash, &mut cmd, "maggraph", &mut stdout),
            ShellChoice::Zsh => generate(shells::Zsh, &mut cmd, "maggraph", &mut stdout),
            ShellChoice::Fish => generate(shells::Fish, &mut cmd, "maggraph", &mut stdout),
            ShellChoice::Elvish => generate(shells::Elvish, &mut cmd, "maggraph", &mut stdout),
            ShellChoice::PowerShell => {
                generate(shells::PowerShell, &mut cmd, "maggraph", &mut stdout)
            }
        }
        return Ok(());
    }

    let resolved = MagGraphConfig::load(&cli.config)?;

    match cli.command {
        Some(Commands::Init(args)) => cmd::init::run(&resolved, &args),
        Some(Commands::Query(args)) => cmd::query::run(&resolved, &args),
        Some(Commands::Sync(args)) => cmd::sync::run(&resolved, &args),
        Some(Commands::Scaffold(args)) => cmd::scaffold::run(&resolved, &args),
        Some(Commands::Ui(args)) => cmd::ui::run(&resolved, &args),
        Some(Commands::Complete { .. }) => unreachable!("handled above"),
        None => {
            tracing::info!(
                mode = ?resolved.config.storage.mode,
                root = %resolved.root_path.display(),
                "loaded configuration; use --help for subcommands"
            );
            Ok(())
        }
    }
}
