mod safe_paths;
mod checkpoint;
mod diff;
mod restore;
mod runner;
mod tui;

use clap::{Parser, Subcommand};
use colored::*;
use std::env;
use std::process;

use checkpoint::Engine;
use diff::DiffEngine;
use restore::RestoreEngine;
use runner::AgentRunner;
use tui::Dashboard;

#[derive(Parser)]
#[command(name = "agent-rewind")]
#[command(about = "Instant zero-overhead copy-on-write undo and time machine for AI coding agents.")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a .rewind repository in the current directory
    Init,

    /// Create a checkpoint of the current workspace state
    Save {
        /// Optional message describing the checkpoint
        #[arg(short, long, default_value = "Manual checkpoint")]
        message: String,

        /// Optional session identifier
        #[arg(short, long)]
        session: Option<String>,
    },

    /// List all captured checkpoints
    List,

    /// Show changes between the current workspace and a checkpoint
    Diff {
        /// Checkpoint ID or 1-based index (defaults to latest)
        #[arg(default_value = "latest")]
        checkpoint: String,

        /// Print full unified line-by-line diff
        #[arg(short, long)]
        full: bool,
    },

    /// Roll back the workspace to an exact checkpoint state
    Restore {
        /// Checkpoint ID or 1-based index to restore
        checkpoint: String,
    },

    /// Wrap and run an agent command with automatic pre/post checkpoints
    Run {
        /// The command and arguments to execute
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },

    /// Show the interactive session dashboard
    Status,
}

fn main() {
    let cli = Cli::parse();
    let current_dir = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let engine = Engine::new(current_dir);

    match cli.command {
        Commands::Init => {
            if let Err(e) = engine.init() {
                eprintln!("{}", format!("Failed to initialize: {}", e).red());
                process::exit(1);
            }
            println!("{}", "✔ Initialized .rewind repository.".green().bold());
        }

        Commands::Save { message, session } => {
            match engine.create_checkpoint(&message, session.as_deref(), None) {
                Ok(cp) => {
                    println!(
                        "{}",
                        format!(
                            "✔ Checkpoint '{}' created! ({} files captured)",
                            cp.id,
                            cp.files.len()
                        )
                        .green()
                        .bold()
                    );
                }
                Err(e) => {
                    eprintln!("{}", format!("Error creating checkpoint: {}", e).red());
                    process::exit(1);
                }
            }
        }

        Commands::List => {
            let dashboard = Dashboard::new(&engine);
            if let Err(e) = dashboard.display() {
                eprintln!("{}", format!("Error displaying dashboard: {}", e).red());
                process::exit(1);
            }
        }

        Commands::Diff { checkpoint, full } => {
            let target_cp = if checkpoint == "latest" {
                match engine.get_latest_checkpoint() {
                    Ok(Some(cp)) => cp,
                    Ok(None) => {
                        eprintln!("{}", "No checkpoints found. Create one first with `agent-rewind save`.".yellow());
                        process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("{}", format!("Error loading checkpoint: {}", e).red());
                        process::exit(1);
                    }
                }
            } else {
                match engine.get_checkpoint(&checkpoint) {
                    Ok(Some(cp)) => cp,
                    Ok(None) => {
                        eprintln!("{}", format!("Checkpoint '{}' not found.", checkpoint).red());
                        process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("{}", format!("Error loading checkpoint: {}", e).red());
                        process::exit(1);
                    }
                }
            };

            println!(
                "{}",
                format!("Comparing workspace against checkpoint '{}' ({}):", target_cp.id, target_cp.message).bold()
            );

            let diff_engine = DiffEngine::new(&engine);
            match diff_engine.compute_working_tree_diff(&target_cp) {
                Ok(deltas) => {
                    diff_engine.print_diff_summary(&deltas);
                    if full && !deltas.is_empty() {
                        if let Err(e) = diff_engine.print_full_unified_diff(&target_cp, &deltas) {
                            eprintln!("{}", format!("Error rendering full diff: {}", e).red());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}", format!("Error computing diff: {}", e).red());
                    process::exit(1);
                }
            }
        }

        Commands::Restore { checkpoint } => {
            let target_cp = match engine.get_checkpoint(&checkpoint) {
                Ok(Some(cp)) => cp,
                Ok(None) => {
                    eprintln!("{}", format!("Checkpoint '{}' not found.", checkpoint).red());
                    process::exit(1);
                }
                Err(e) => {
                    eprintln!("{}", format!("Error loading checkpoint: {}", e).red());
                    process::exit(1);
                }
            };

            let restore_engine = RestoreEngine::new(&engine);
            if let Err(e) = restore_engine.restore_checkpoint(&target_cp) {
                eprintln!("{}", format!("Error restoring checkpoint: {}", e).red());
                process::exit(1);
            }
        }

        Commands::Run { command } => {
            let runner = AgentRunner::new(&engine);
            match runner.execute(&command) {
                Ok(code) => process::exit(code),
                Err(e) => {
                    eprintln!("{}", format!("Execution error: {}", e).red());
                    process::exit(1);
                }
            }
        }

        Commands::Status => {
            let dashboard = Dashboard::new(&engine);
            if let Err(e) = dashboard.display() {
                eprintln!("{}", format!("Error: {}", e).red());
                process::exit(1);
            }
        }
    }
}
