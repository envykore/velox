//! velox - static analyzer and build optimizer for rojo/luau projects.
//! finds dead modules, circular requires, oversized scripts, debug noise
//! and strips it before you sync to studio

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
    /// strip debug calls and write optimized output
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

            if !cfg.build.strip_debug {
                println!("\nstrip_debug is off, nothing to do");
                return Ok(());
            }

            println!("\n{}", "stripping...".bold());
            let mut total_calls = 0;
            let mut total_blocks = 0;
            let mut written = 0;

            for file in tree.scripts() {
                let Some(source) = &file.source else { continue };
                let result = strip::process(source, &cfg.build);
                total_calls += result.calls_removed;
                total_blocks += result.dev_blocks_removed;

                let rel = file.path.strip_prefix(&tree.root).unwrap_or(&file.path);
                let dest = out.join(rel);
                if let Some(p) = dest.parent() { std::fs::create_dir_all(p)?; }
                std::fs::write(&dest, result.source.as_bytes())?;
                written += 1;
            }

            println!(
                "  {} calls, {} __DEV__ blocks removed across {} files -> {:?}",
                total_calls, total_blocks, written, out
            );
        }
    }

    Ok(())
}