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

fn deserialize_level_filter<'de, D>(deserializer: D) -> Result<Option<LevelFilter>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        None => Ok(None),
        Some(s) => s
            .parse::<LevelFilter>()
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    #[serde(default = "default_api_url")]
    pub api_url: String,
    #[serde(default = "default_content_url")]
    pub content_url: String,
    #[serde(default)]
    pub gpu: bool,
    #[serde(default, deserialize_with = "deserialize_level_filter")]
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
        match config_path() {
            Some(path) => {
                let content = std::fs::read_to_string(&path)?;
                let config: Config = toml::from_str(&content)?;
                Ok(config)
            }
            None => Ok(Config::default()),
        }
    }
}
