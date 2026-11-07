//! config.rs
//! reads `velox.toml` from the project root if it exists. else
//! just uses defaults
//! shouldnt crash if the file isnt there 

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

    // paths to skip entirely, relative to project root
    #[serde(default)]
    pub ignore: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct BuildConfig {
    #[serde(default)]
    pub strip_debug: bool,

    #[serde(default)]
    pub inline_constants: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            analysis: AnalysisConfig::default(),
            build: BuildConfig::default(),
        }
    }
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            large_script_threshold_kb: default_large_script_kb(),
            ignore: Vec::new(),
        }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            strip_debug: false,
            inline_constants: false,
        }
    }
}

fn default_large_script_kb() -> usize {
    50
}

pub fn load(project_root: &Path) -> Result<Config> {
    let config_path = project_root.join("velox.toml");

    if !config_path.exists() {
        return Ok(Config::default());
    }

    let raw = std::fs::read_to_string(&config_path)?;
    let config: Config = toml::from_str(&raw)?;
    Ok(config)
}