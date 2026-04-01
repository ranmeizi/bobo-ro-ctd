use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub block_threshold: usize,
    pub counting_seconds: usize,
    pub play_voice: bool,
    pub block_min_width: usize,
    pub block_max_width: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            block_threshold: 60,
            counting_seconds: 29,
            play_voice: false,
            block_min_width: 5,
            block_max_width: 50,
        }
    }
}

pub fn config_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            return parent.join("config").join("config.toml");
        }
    }
    PathBuf::from("config").join("config.toml")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(content) => toml::from_str::<AppConfig>(&content).unwrap_or_else(|_| AppConfig::default()),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(cfg: &AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create config dir failed: {e}"))?;
    }
    let content = toml::to_string_pretty(cfg).map_err(|e| format!("serialize config failed: {e}"))?;
    fs::write(&path, content).map_err(|e| format!("write config failed: {e}"))?;
    Ok(())
}
