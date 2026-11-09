//! velox - static analyzer and build optimizer for rojo/luau projects.
//! finds dead modules, circular requires, oversized scripts, debug noise
//! and strips it before you sync to studio

mod build;
mod config;
mod graph;
mod project;
mod report;
mod strip;
mod tree;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "velox", about = "build-time luau optimizer for roblox projects")]
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
        /// exit 1 if issues found, useful for ci
        #[arg(long)]
        ci: bool,
    },
    /// build optimized output - strips debug calls and copies full project to dist
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
            let issues = report::print_analysis(&tree, &graph, &cfg);

            if ci && issues > 0 {
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

            println!("\n{}", "building...".bold());
            let result = build::run(&tree, &project, &out, &cfg)?;
            println!(
                "  {} files written, {} calls removed, {} __DEV__ blocks removed, {} bytes saved -> {:?}",
                result.files_written, result.calls_removed, result.dev_blocks_removed, result.bytes_saved, out
            );
        }
    }

    Ok(())
}