use serde::{Deserialize, Serialize};
use serde_json;
use std::env;
use std::path;
use std::path::PathBuf;
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
    // get position relative to working directory
    let curr_dir = match env::current_dir() {
        Ok(d) => d,
        _ => return Err("parent directory of config not found".to_string()),
    };

    let config_path = match path::absolute(curr_dir.join(filepath)) {
        Ok(pb) => pb,
        Err(e) => return Err(e.to_string()),
    };

    let parent_dir = match config_path.parent() {
        Some(p) => p.to_path_buf(),
        _ => return Err("parent directory of config not found".to_string()),
    };

    let json_as_str = match fs::read_to_string(&config_path).await {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };

    let config: Config = match serde_json::from_str(&json_as_str) {
        Ok(j) => j,
        Err(e) => return Err(e.to_string()),
    };

    // create absolute filepaths for key and cert
    let key_filepath = match path::absolute(parent_dir.join(&config.key_filepath)) {
        Ok(j) => j,
        Err(e) => return Err(e.to_string()),
    };

    if !key_filepath.is_file() {
        return Err(
            "failed to create absolute path from relative path for key_filepath".to_string(),
        );
    }

    let cert_filepath = match path::absolute(parent_dir.join(&config.cert_filepath)) {
        Ok(j) => j,
        Err(e) => return Err(e.to_string()),
    };

    if !cert_filepath.is_file() {
        return Err(
            "failed to create absolute path from relative path for cert_filepath".to_string(),
        );
    }

    Ok(Config {
        host_and_port: config.host_and_port,
        key_filepath: key_filepath,
        cert_filepath: cert_filepath,
        addresses: config.addresses,
        dangerous_self_signed_addresses: config.dangerous_self_signed_addresses,
    })
}
