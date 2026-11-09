//! build.rs
//! copies the full project to an output directory, stripping debug calls
//! from scripts along the way. everything else (models, project file, etc)
//! gets copied verbatim so the output is a complete rojo project

use crate::config::Config;
use crate::strip;
use crate::tree::SourceTree;
use anyhow::Result;
use std::path::Path;
use walkdir::WalkDir;

pub struct BuildResult {
    pub files_written: usize,
    pub calls_removed: usize,
    pub dev_blocks_removed: usize,
    pub bytes_saved: usize,
}

pub fn run(tree: &SourceTree, project_path: &Path, out: &Path, cfg: &Config) -> Result<BuildResult> {
    let root = &tree.root;
    let mut result = BuildResult {
        files_written: 0,
        calls_removed: 0,
        dev_blocks_removed: 0,
        bytes_saved: 0,
    };

    // copy the project file itself
    copy_file(project_path, &out.join("default.project.json"))?;

    // walk everything under root
    for entry in WalkDir::new(root).into_iter().flatten() {
        if !entry.file_type().is_file() { continue; }

        let src_path = entry.path();
        let rel = src_path.strip_prefix(root).unwrap_or(src_path);
        let dest = out.join(rel);

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let kind = crate::tree::FileKind::from_path(src_path);

        if kind.is_script() && cfg.build.strip_debug {
            let source = std::fs::read_to_string(src_path)?;
            let original_len = source.len();
            let stripped = strip::process(&source, &cfg.build);

            result.calls_removed += stripped.calls_removed;
            result.dev_blocks_removed += stripped.dev_blocks_removed;
            result.bytes_saved += original_len.saturating_sub(stripped.source.len());

            std::fs::write(&dest, stripped.source.as_bytes())?;
        } else {
            // verbatim copy - models, json, assets, anything else
            std::fs::copy(src_path, &dest)?;
        }

        result.files_written += 1;
    }

    Ok(result)
}

fn copy_file(src: &Path, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(src, dest)?;
    Ok(())
}