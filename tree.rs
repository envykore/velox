//! tree.rs
//! walks the filesystem based on what the rojo project tells us and builds
//! a flat list of every source file we care about also reads script contents
//! into memory since we'll need them for the graph later anyway

use crate::project::{self, RojoNode, RojoProject};
use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// u32 is plenty, no roblox project has 4 billion scripts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);

#[derive(Debug)]
pub struct SourceFile {
    pub id: FileId,
    pub path: PathBuf,
    pub kind: FileKind,
    // only Some for scripts, not models/etc 
    pub source: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    // flat vec, FileId(i) == files[i]
    pub files: Vec<SourceFile>,
    pub root: PathBuf,
}

impl SourceTree {
    pub fn get(&self, id: FileId) -> &SourceFile {
        &self.files[id.0 as usize]
    }

    pub fn scripts(&self) -> impl Iterator<Item = &SourceFile> {
        self.files.iter().filter(|f| f.kind.is_script())
    }

    pub fn total_script_bytes(&self) -> usize {
        self.scripts()
            .filter_map(|f| f.source.as_ref())
            .map(|s| s.len())
            .sum()
    }

    // path ---> id lookup, this is used during graph build to resolve requires
    pub fn find_by_path(&self, path: &Path) -> Option<FileId> {
        self.files.iter().find(|f| f.path == path).map(|f| f.id)
    }
}

pub fn build(project: &RojoProject, project_path: &Path) -> Result<SourceTree> {
    let root = project::root_dir(project_path);
    let mut paths: Vec<(PathBuf, FileKind)> = Vec::new();

    collect_node(&project.tree, &root, &mut paths);

    let files = paths
        .into_iter()
        .enumerate()
        .map(|(i, (path, kind))| {
            let source = if kind.is_script() {
                std::fs::read_to_string(&path).ok()
            } else {
                None
            };
            SourceFile { id: FileId(i as u32), path, kind, source }
        })
        .collect();

    Ok(SourceTree { files, root })
}

fn collect_node(node: &RojoNode, base: &Path, out: &mut Vec<(PathBuf, FileKind)>) {
    if let Some(rel_path) = &node.path {
        let full = base.join(rel_path);
        if full.is_dir() {
            collect_dir(&full, out);
        } else if full.is_file() {
            let kind = FileKind::from_path(&full);
            out.push((full, kind));
        }
        // if neither the path plobabry doesnt exist
    }

    for child in node.children.values() {
        collect_node(child, base, out);
    }
}

fn collect_dir(dir: &Path, out: &mut Vec<(PathBuf, FileKind)>) {
    for entry in WalkDir::new(dir).into_iter().flatten() {
        if entry.file_type().is_file() {
            let kind = FileKind::from_path(entry.path());
            out.push((entry.path().to_path_buf(), kind));
        }
    }
}