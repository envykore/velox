//! project.rs
//! handles loading and making sense of the rojo project file (default.project.json)
//! rojo's format is a bit weird with the $className/$path stuff mixed in with
//! child nodes so we have to flatten it carefully

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct RojoProject {
    pub name: String,
    pub tree: RojoNode,
}

// rojo nodes are weird - $path and $className are special keys but
// everything else is just a child node, hence the flatten
#[derive(Debug, Deserialize, Clone)]
pub struct RojoNode {
    #[serde(rename = "$className")]
    pub class_name: Option<String>,

    #[serde(rename = "$path")]
    pub path: Option<String>,

    #[serde(flatten)]
    pub children: HashMap<String, RojoNode>,
}

pub fn load(project_path: &Path) -> Result<RojoProject> {
    let raw = std::fs::read_to_string(project_path)
        .with_context(|| format!("couldnt read {:?}", project_path))?;

    serde_json::from_str(&raw)
        .with_context(|| format!("{:?} doesnt look like a valid rojo project", project_path))
}

pub fn root_dir(project_path: &Path) -> PathBuf {
    project_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf()
}