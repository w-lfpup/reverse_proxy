use http::Uri;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
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
    if key_filepath.is_dir() {
        return Err(
            "failed to create absolute path from relative path for key_filepath".to_string(),
        );
    }

    let cert_filepath = match path::absolute(parent_dir.join(&config.cert_filepath)) {
        Ok(j) => j,
        Err(e) => return Err(e.to_string()),
    };
    if cert_filepath.is_dir() {
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

pub fn get_host_and_port(uri: &Uri) -> Option<String> {
    let host = match uri.host() {
        Some(h) => h,
        _ => return None,
    };

    let port = match uri.port() {
        Some(p) => p.to_string(),
        _ => {
            let scheme = match uri.scheme() {
                Some(h) => h.as_str(),
                _ => "http",
            };

            match scheme {
                "https" => "443".to_string(),
                _ => "80".to_string(),
            }
        }
    };

    Some(host.to_string() + ":" + &port)
}

pub fn create_address_map(config: &Config) -> Result<HashMap<String, (Uri, bool)>, String> {
    let mut hashmap = HashMap::<String, (Uri, bool)>::new();
    if let Err(e) = add_addresses_to_map(&mut hashmap, &config.addresses, false) {
        return Err(e);
    };

    if let Some(self_signed_addresses) = &config.dangerous_self_signed_addresses {
        if let Err(e) = add_addresses_to_map(&mut hashmap, &self_signed_addresses, true) {
            return Err(e);
        };
    };

    Ok(hashmap)
}

fn add_addresses_to_map<'a>(
    url_map: &mut HashMap<String, (Uri, bool)>,
    addresses: &Vec<(String, String)>,
    is_dangerous: bool,
) -> Result<(), String> {
    for (source_str, target_str) in addresses {
        let source_uri = match Uri::try_from(source_str) {
            Ok(uri) => uri,
            Err(e) => return Err(e.to_string()),
        };

        // get port if available
        let host = match get_host_and_port(&source_uri) {
            Some(h) => h,
            _ => return Err("could not parse host and port from address".to_string()),
        };

        let target_uri = match Uri::try_from(target_str) {
            Ok(uri) => uri,
            Err(e) => return Err(e.to_string()),
        };

        let path_and_query = match get_target_uri(&target_uri) {
            Ok(p_q) => p_q,
            Err(e) => return Err(e),
        };

        let mut target_parts = target_uri.clone().into_parts();
        target_parts.path_and_query = Some(path_and_query);

        url_map.insert(host, (target_uri, is_dangerous));
    }

    Ok(())
}

pub fn get_target_uri<'a>(dest_uri: &Uri) -> Result<http::uri::PathAndQuery, String> {
    let mut uri_path = path::Path::new(dest_uri.path());
    if uri_path.is_file() {
        uri_path = match uri_path.parent() {
            Some(uri) => uri,
            _ => return Err("path has no parent path".to_string()),
        }
    }

    let uri_path_str = uri_path.to_string_lossy().to_string();
    match http::uri::PathAndQuery::try_from(uri_path_str) {
        Ok(p_q) => Ok(p_q),
        Err(e) => return Err(e.to_string()),
    }
}
