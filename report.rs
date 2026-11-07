//! report.rs
//! takes the analysis results and prints them in a readable way
//! nothing fancy 

use crate::graph::RequireGraph;
use crate::tree::SourceTree;
use colored::Colorize;
use std::cmp::Reverse;

// NOTE: highly arbitrary number but i think scripts bigger than 50kb 
// are probably going to be doing too much in one place
const LARGE_SCRIPT_KB: usize = 50;

pub fn print_analysis(tree: &SourceTree, graph: &RequireGraph) {
    println!("{}", "\n=== velox ===\n".bold());

    let script_count = tree.scripts().count();
    let total_kb = tree.total_script_bytes() / 1024;

    println!("{}", "files".underline());
    println!("  scripts:      {}", script_count);
    println!("  total source: {} kb\n", total_kb);

    let mut large: Vec<_> = tree
        .scripts()
        .filter(|f| {
            f.source.as_ref().map(|s| s.len() / 1024 >= LARGE_SCRIPT_KB).unwrap_or(false)
        })
        .collect();
    large.sort_by_key(|f| Reverse(f.source.as_ref().map(|s| s.len()).unwrap_or(0)));

    if !large.is_empty() {
        println!("{}", format!("large scripts (>{}kb)", LARGE_SCRIPT_KB).underline().yellow());
        for f in &large {
            let kb = f.source.as_ref().map(|s| s.len() / 1024).unwrap_or(0);
            println!("  {:>5} kb  {:?}", kb, f.path.file_name().unwrap_or_default());
        }
        println!();
    }

    // filter out server/local scripts
    let dead = graph.unreferenced_modules(tree);
    if !dead.is_empty() {
        println!("{}", "dead modules (never required)".underline().yellow());
        for id in &dead {
            let file = tree.get(*id);
            println!("  {:?}", file.path.file_name().unwrap_or_default());
        }
        println!();
    }

    let cycles = graph.find_cycles();
    if !cycles.is_empty() {
        println!("{}", "circular requires".underline().red());
        for cycle in &cycles {
            let names: Vec<_> = cycle
                .iter()
                .map(|&id| {
                    tree.get(id)
                        .path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                })
                .collect();
            println!("  {} -> {}", names.join(" -> "), names[0]);
        }
        println!();
    }

    let issues = large.len() + dead.len() + cycles.len();
    if issues == 0 {
        println!("{}", "no issues found".green());
    } else {
        println!("{}", format!("{} issue(s) across {} scripts", issues, script_count).yellow());
    }
}