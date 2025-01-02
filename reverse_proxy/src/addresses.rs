use http::Uri;
use std::collections::HashMap;
use std::env;
use std::path;
use std::path::PathBuf;
use tokio::fs;

use config::Config;

pub fn get_host() {
    
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

        // forgot to update the target uri?
        let updated_target_uri = match Uri::from_parts(target_parts) {
            Ok(p_q) => p_q,
            Err(e) => return Err(e.to_string()),
        };

        url_map.insert(host, (updated_target_uri, is_dangerous));
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
