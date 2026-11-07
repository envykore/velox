//! graph.rs
//! builds a require graph from all the scripts we found. the tricky part is
//! that luau require() calls can be dynamic (require(script.Parent.Whatever))
//! which we cant statically resolve (skip)

use crate::tree::{FileId, FileKind, SourceTree};
use std::collections::HashSet;

pub struct GraphNode {
    pub id: FileId,
    // resolved ids of modules this file requires
    pub requires: Vec<FileId>,
    pub required_by: Vec<FileId>,
}

pub struct RequireGraph {
    // indexed by FileId, same order as SourceTree
    pub nodes: Vec<GraphNode>,
}

impl RequireGraph {
    pub fn get(&self, id: FileId) -> &GraphNode {
        &self.nodes[id.0 as usize]
    }

    // modules that nothing requires - either entry points or dead code
    // caller has to filter out server/local scripts since those are always "unreferenced"
    pub fn unreferenced_modules<'a>(&'a self, tree: &'a SourceTree) -> Vec<FileId> {
        self.nodes
            .iter()
            .filter(|n| {
                n.required_by.is_empty()
                    && tree.get(n.id).kind == FileKind::ModuleScript
            })
            .map(|n| n.id)
            .collect()
    }

    pub fn find_cycles(&self) -> Vec<Vec<FileId>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = Vec::new();

        for node in &self.nodes {
            self.dfs(node.id, &mut visited, &mut stack, &mut cycles);
        }

        cycles
    }

    fn dfs(
        &self,
        id: FileId,
        visited: &mut HashSet<FileId>,
        stack: &mut Vec<FileId>,
        cycles: &mut Vec<Vec<FileId>>,
    ) {
        if let Some(pos) = stack.iter().position(|&s| s == id) {
            cycles.push(stack[pos..].to_vec());
            return;
        }
        if visited.contains(&id) {
            return;
        }

        visited.insert(id);
        stack.push(id);

        for &dep in &self.get(id).requires {
            self.dfs(dep, visited, stack, cycles);
        }

        stack.pop();
    }
}

pub fn build(tree: &SourceTree) -> anyhow::Result<RequireGraph> {
    let mut nodes: Vec<GraphNode> = tree
        .files
        .iter()
        .map(|f| GraphNode { id: f.id, requires: Vec::new(), required_by: Vec::new() })
        .collect();

    // collect edges first so we're not borrowing nodes while mutating it
    let edges: Vec<(FileId, Vec<FileId>)> = tree
        .scripts()
        .map(|file| {
            let requires = file
                .source
                .as_deref()
                .map(|src| resolve_requires(src, file.id, tree))
                .unwrap_or_default();
            (file.id, requires)
        })
        .collect();

    // second pass to wire up back-refs
    for (src_id, deps) in edges {
        for &dep_id in &deps {
            nodes[dep_id.0 as usize].required_by.push(src_id);
        }
        nodes[src_id.0 as usize].requires = deps;
    }

    Ok(RequireGraph { nodes })
}

// not a real parser just scans for require("...") patterns with string literals
// dynamic requires like require(script.Parent.X) 
fn resolve_requires(source: &str, from: FileId, tree: &SourceTree) -> Vec<FileId> {
    let mut results = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("--") {
            continue;
        }

        let mut rest = trimmed;
        while let Some(pos) = rest.find("require(") {
            let after = &rest[pos + 8..];
            if let Some(raw) = extract_string_arg(after) {
                if let Some(id) = resolve_path(raw, from, tree) {
                    results.push(id);
                }
            }
            rest = &rest[pos + 8..];
        }
    }

    results
}

fn extract_string_arg(s: &str) -> Option<&str> {
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
    Some(&rest[..end])
}

fn resolve_path(req: &str, from: FileId, tree: &SourceTree) -> Option<FileId> {
    let parent = tree.get(from).path.parent()?;

    let luau = parent.join(req).with_extension("luau");
    if let Some(id) = tree.find_by_path(&luau) { return Some(id); }

    let lua = parent.join(req).with_extension("lua");
    tree.find_by_path(&lua)
}