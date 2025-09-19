use hyper::Uri;
use std::collections::HashMap;

use crate::config::Config;

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

fn add_addresses_to_map(
    url_map: &mut HashMap<String, (Uri, bool)>,
    addresses: &Vec<(String, String)>,
    is_dangerous: bool,
) -> Result<(), String> {
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

        url_map.insert(source_host.to_string(), (target_uri, is_dangerous));
    }

    Ok(())
}
