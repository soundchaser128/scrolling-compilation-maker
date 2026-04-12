use std::path::PathBuf;

use color_eyre::Result;
use serde::Deserialize;
use tracing::level_filters::LevelFilter;

fn default_api_url() -> String {
    "https://alexandria.soundchaser128.com".to_string()
}

fn default_content_url() -> String {
    "https://content.r2.soundchaser128.com".to_string()
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct RawConfig {
    #[serde(default = "default_api_url")]
    api_url: String,
    #[serde(default = "default_content_url")]
    content_url: String,
    #[serde(default)]
    gpu: bool,
    #[serde(default)]
    log: Option<String>,
}

#[derive(Debug)]
pub struct Config {
    pub api_url: String,
    pub content_url: String,
    pub gpu: bool,
    pub log: Option<LevelFilter>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: default_api_url(),
            content_url: default_content_url(),
            gpu: false,
            log: None,
        }
    }
}

fn config_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let path = dir.join("config.toml");
    if path.exists() { Some(path) } else { None }
}

impl Config {
    pub fn load() -> Result<Self> {
        let raw = match config_path() {
            Some(path) => {
                let content = std::fs::read_to_string(&path)?;
                toml::from_str::<RawConfig>(&content)?
            }
            None => RawConfig::default(),
        };
        let log = raw
            .log
            .map(|s| s.parse::<LevelFilter>())
            .transpose()
            .map_err(|e| color_eyre::eyre::eyre!("invalid log level in config: {e}"))?;
        Ok(Config {
            api_url: raw.api_url,
            content_url: raw.content_url,
            gpu: raw.gpu,
            log,
        })
    }
}
