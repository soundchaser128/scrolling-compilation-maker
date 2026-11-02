use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub api_url: String,
    pub content_url: String,
    pub bind_addr: String,
    pub output_dir: PathBuf,
    pub gpu: bool,
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            api_url: env_or(
                "ALEXANDRIA_API_URL",
                "https://alexandria.soundchaser128.com",
            ),
            content_url: env_or("CONTENT_URL", "https://content.r2.soundchaser128.com"),
            bind_addr: env_or("BIND_ADDR", "127.0.0.1:3001"),
            output_dir: PathBuf::from(env_or("OUTPUT_DIR", "./output")),
            gpu: env_or("GPU", "false") == "true",
        }
    }
}
