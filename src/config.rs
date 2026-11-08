//! loads velox.toml from project root, falls back to defaults if missing

use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub analysis: AnalysisConfig,
    #[serde(default)]
    pub build: BuildConfig,
}

#[derive(Debug, Deserialize)]
pub struct AnalysisConfig {
    #[serde(default = "default_large_script_kb")]
    pub large_script_threshold_kb: usize,
    #[serde(default)]
    pub ignore: Vec<String>, // relative to project root
}

#[derive(Debug, Deserialize)]
pub struct BuildConfig {
    #[serde(default)]
    pub strip_debug: bool,
    #[serde(default = "default_strip_calls")]
    pub strip_calls: Vec<String>,
    #[serde(default)]
    pub inline_constants: bool,
}

fn default_large_script_kb() -> usize { 50 }
fn default_strip_calls() -> Vec<String> {
    vec!["print".to_string(), "warn".to_string()]
}

impl Default for Config {
    fn default() -> Self {
        Self { analysis: AnalysisConfig::default(), build: BuildConfig::default() }
    }
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self { large_script_threshold_kb: default_large_script_kb(), ignore: Vec::new() }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self { strip_debug: false, strip_calls: default_strip_calls(), inline_constants: false }
    }
}

pub fn load(project_root: &Path) -> Result<Config> {
    let path = project_root.join("velox.toml");
    if !path.exists() { return Ok(Config::default()); }
    let raw = std::fs::read_to_string(&path)?;
    Ok(toml::from_str(&raw)?)
}