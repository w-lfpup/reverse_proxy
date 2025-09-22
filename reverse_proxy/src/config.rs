use serde::{Deserialize, Serialize};
use serde_json;
use std::path::PathBuf;
use std::{env, path};
use tokio::fs;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Config {
    pub host_and_port: String,
    pub key_filepath: PathBuf,
    pub cert_filepath: PathBuf,
    pub addresses: Vec<(String, String)>,
    pub dangerous_self_signed_addresses: Option<Vec<(String, String)>>,
}

pub async fn from_filepath(filepath: &PathBuf) -> Result<Config, String> {
    let config_path = match path::absolute(filepath) {
        Ok(pb) => pb,
        Err(e) => return Err(e.to_string()),
    };

    let json_as_str = match fs::read_to_string(&config_path).await {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };

    let parent_dir = match config_path.parent() {
        Some(p) => p.to_path_buf(),
        _ => return Err("parent directory of config not found".to_string()),
    };

    let mut config: Config = match serde_json::from_str(&json_as_str) {
        Ok(j) => j,
        Err(e) => return Err(e.to_string()),
    };

    config.key_filepath = parent_dir.join(&config.key_filepath);
    config.cert_filepath = parent_dir.join(&config.cert_filepath);

    Ok(config)
}
