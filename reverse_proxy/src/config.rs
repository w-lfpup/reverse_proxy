use serde::{Deserialize, Serialize};
use serde_json;
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
    let json_as_str = match fs::read_to_string(&filepath).await {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };

    match serde_json::from_str(&json_as_str) {
        Ok(j) => j,
        Err(e) => return Err(e.to_string()),
    }
}
