//! graph.rs
//! builds a require graph from all the scripts we found. the tricky part is
//! that luau require() calls can be dynamic (require(script.Parent.Whatever))
//! which we cant statically resolve (skip)

use crate::tree::SourceTree;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub struct GraphNode {
    pub path: PathBuf,
    pub requires: Vec<String>,
    pub is_required_by: Vec<PathBuf>,
}

pub struct RequireGraph {
    pub nodes: HashMap<PathBuf, GraphNode>,
}

impl RequireGraph {
    // modules that nothing requires - either entry points or dead code
    // caller has to filter out server/local scripts since those are always "unreferenced"
    pub fn unreferenced_modules(&self) -> Vec<&GraphNode> {
        self.nodes
            .values()
            .filter(|n| n.is_required_by.is_empty())
            .collect()
    }

    pub fn find_cycles(&self) -> Vec<Vec<PathBuf>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = Vec::new();

        for start in self.nodes.keys() {
            self.dfs(start, &mut visited, &mut stack, &mut cycles);
        }

        cycles
    }

    fn dfs(
        &self,
        node: &PathBuf,
        visited: &mut HashSet<PathBuf>,
        stack: &mut Vec<PathBuf>,
        cycles: &mut Vec<Vec<PathBuf>>,
    ) {
        if let Some(pos) = stack.iter().position(|p| p == node) {
            cycles.push(stack[pos..].to_vec());
            return;
        }
        if visited.contains(node) {
            return;
        }

        visited.insert(node.clone());
        stack.push(node.clone());

        if let Some(n) = self.nodes.get(node) {
            for req in &n.requires {
                if let Some(resolved) = resolve_require(req, node) {
                    self.dfs(&resolved, visited, stack, cycles);
                }
            }
        }

        stack.pop();
    }
}

pub fn build(tree: &SourceTree) -> anyhow::Result<RequireGraph> {
    let mut nodes: HashMap<PathBuf, GraphNode> = HashMap::new();

    for file in tree.scripts() {
        let requires = file
            .source
            .as_deref()
            .map(extract_requires)
            .unwrap_or_default();

        nodes.insert(
            file.path.clone(),
            GraphNode {
                path: file.path.clone(),
                requires,
                is_required_by: Vec::new(),
            },
        );
    }

    // second pass to wire up back-refs
    let paths: Vec<PathBuf> = nodes.keys().cloned().collect();
    for src in &paths {
        let reqs: Vec<String> = nodes[src].requires.clone();
        for req in reqs {
            if let Some(target) = resolve_require(&req, src) {
                if let Some(target_node) = nodes.get_mut(&target) {
                    target_node.is_required_by.push(src.clone());
                }
            }
        }
    }

    Ok(RequireGraph { nodes })
}

// not a real parser just scans for require("...") patterns with string literals
// dynamic requires like require(script.Parent.X) 
fn extract_requires(source: &str) -> Vec<String> {
    let mut results = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("--") {
            continue;
        }

        let mut rest = trimmed;
        while let Some(pos) = rest.find("require(") {
            let after = &rest[pos + 8..];
            if let Some(arg) = extract_string_arg(after) {
                results.push(arg);
            }
            rest = &rest[pos + 8..];
        }
    }

    results
}

fn extract_string_arg(s: &str) -> Option<String> {
    let s = s.trim_start();
    let (quote, rest) = if s.starts_with('"') {
        ('"', &s[1..])
    } else if s.starts_with('\'') {
        ('\'', &s[1..])
    } else {
        // cant resolve bc not a literla string 
        return None;
    };

    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

fn resolve_require(req: &str, from: &PathBuf) -> Option<PathBuf> {
    let parent = from.parent()?;

    let luau = parent.join(req).with_extension("luau");
    if luau.exists() { return Some(luau); }

    let lua = parent.join(req).with_extension("lua");
    if lua.exists() { return Some(lua); }

    None
}