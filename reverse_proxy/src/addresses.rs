use config::Config;
use hyper::body::Incoming;
use hyper::header::HOST;
use hyper::Request;
use hyper::Uri;
use std::collections::HashMap;
use std::path;

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

pub fn get_host(req: &Request<Incoming>) -> Option<String> {
    // more durable to say
    // match req.version()

    // http2
    if let Some(host) = req.uri().host() {
        return Some(host.to_string());
    };

    // http 1
    let host_header = match req.headers().get(HOST) {
        Some(h) => h,
        _ => return None,
    };

    let host_str = match host_header.to_str() {
        Ok(hs) => hs,
        _ => return None,
    };

    let host_as_uri = match Uri::try_from(host_str) {
        Ok(hau) => hau,
        _ => return None,
    };

    if let Some(host) = host_as_uri.host() {
        return Some(host.to_string());
    }

    None
}

pub fn create_address_map(config: &Config) -> Result<HashMap<String, (Uri, bool)>, String> {
    // get host incoming
    let mut hashmap = HashMap::<String, (Uri, bool)>::new();
    if let Err(e) = add_addresses_to_map(&mut hashmap, &config.addresses, false) {
        return Err(e);
    };

    // get (host and port, is dangerous) outgoing
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
    // get uri for source, get host

    // get uri for target, get host and port
    for (source_str, target_str) in addresses {
        let source_uri = match Uri::try_from(source_str) {
            Ok(uri) => uri,
            Err(e) => return Err(e.to_string()),
        };

        let source_host = match source_uri.host() {
            Some(h) => h,
            _ => return Err("could not parse host from source uri".to_string()),
        };

        let target_uri = match Uri::try_from(target_str) {
            Ok(uri) => uri,
            Err(e) => return Err(e.to_string()),
        };

        // remove path?

        url_map.insert(source_host.to_string(), (target_uri, is_dangerous));
    }

    Ok(())
}

// for combined paths
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
