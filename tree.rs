//! tree.rs
//! walks the filesystem based on what the rojo project tells us and builds
//! a flat list of every source file we care about also reads script contents
//! into memory since we'll need them for the graph later anyway

use crate::project::{self, RojoNode, RojoProject};
use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    pub kind: FileKind,
    // only Some for scripts, not models/etc 
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileKind {
    Script,       // .server.lua/luau
    LocalScript,  // .client.lua/luau
    ModuleScript, // plain .lua/luau
    Model,        // .rbxm/.rbxmx
    Other,
}

impl FileKind {
    fn from_path(p: &Path) -> Self {
        let name = p.file_name().unwrap_or_default().to_string_lossy();

        // order matters here do not change pls
        if name.ends_with(".server.lua") || name.ends_with(".server.luau") {
            FileKind::Script
        } else if name.ends_with(".client.lua") || name.ends_with(".client.luau") {
            FileKind::LocalScript
        } else if name.ends_with(".lua") || name.ends_with(".luau") {
            FileKind::ModuleScript
        } else if name.ends_with(".rbxm") || name.ends_with(".rbxmx") {
            FileKind::Model
        } else {
            FileKind::Other
        }
    }

    pub fn is_script(&self) -> bool {
        matches!(self, FileKind::Script | FileKind::LocalScript | FileKind::ModuleScript)
    }
}

pub struct SourceTree {
    pub files: Vec<SourceFile>,
    pub root: PathBuf,
}

impl SourceTree {
    pub fn scripts(&self) -> impl Iterator<Item = &SourceFile> {
        self.files.iter().filter(|f| f.kind.is_script())
    }

    pub fn total_script_bytes(&self) -> usize {
        self.scripts()
            .filter_map(|f| f.source.as_ref())
            .map(|s| s.len())
            .sum()
    }
}

pub fn build(project: &RojoProject, project_path: &Path) -> Result<SourceTree> {
    let root = project::root_dir(project_path);
    let mut files: Vec<SourceFile> = Vec::new();

    collect_node(&project.tree, &root, &mut files);

    Ok(SourceTree { files, root })
}

fn collect_node(node: &RojoNode, base: &Path, out: &mut Vec<SourceFile>) {
    if let Some(rel_path) = &node.path {
        let full = base.join(rel_path);
        if full.is_dir() {
            collect_dir(&full, out);
        } else if full.is_file() {
            out.push(load_file(&full));
        }
        // if neither the path plobabry doesnt exist
    }

    for child in node.children.values() {
        collect_node(child, base, out);
    }
}

fn collect_dir(dir: &Path, out: &mut Vec<SourceFile>) {
    for entry in WalkDir::new(dir).into_iter().flatten() {
        if entry.file_type().is_file() {
            out.push(load_file(entry.path()));
        }
    }
}

fn load_file(path: &Path) -> SourceFile {
    let kind = FileKind::from_path(path);
    let source = if kind.is_script() {
        std::fs::read_to_string(path).ok()
    } else {
        None
    };

    SourceFile { path: path.to_path_buf(), kind, source }
}