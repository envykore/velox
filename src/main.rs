//! velox - fast build-time luau optimizer for roblox projects
//! 
//! runs static analysis on your rojo project before you sync to studio.
//! finds dead modules, circular requires, oversized scripts, debug noise.
//! no runtime overhead, no guessing - just reads your files directly

mod config;
mod project;
mod tree;
mod graph;
mod report;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "velox", about = "fast build-time luau optimizer for roblox projects")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// analyze a rojo project and report issues
    Analyze {
        #[arg(default_value = "default.project.json")]
        project: PathBuf,

        /// exit with code 1 if issues are found (useful for ci)
        #[arg(long)]
        ci: bool,
    },
    /// build an optimized copy of the project
    Build {
        #[arg(default_value = "default.project.json")]
        project: PathBuf,

        #[arg(short, long, default_value = "dist")]
        out: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Analyze { project, ci } => {
            let root = project::root_dir(&project);
            let cfg = config::load(&root)?;
            let proj = project::load(&project)?;
            let tree = tree::build(&proj, &project, &cfg)?;
            let graph = graph::build(&tree)?;
            report::print_analysis(&tree, &graph, &cfg);

            if ci {
                std::process::exit(1);
            }
        }
        Command::Build { project, out } => {
            let root = project::root_dir(&project);
            let cfg = config::load(&root)?;
            let proj = project::load(&project)?;
            let tree = tree::build(&proj, &project, &cfg)?;
            let graph = graph::build(&tree)?;
            report::print_analysis(&tree, &graph, &cfg);
            println!("\nbuild output to {:?} - not implemented yet", out);
        }
    }

    Ok(())
}