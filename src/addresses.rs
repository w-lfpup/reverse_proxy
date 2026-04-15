use crate::config::Config;
use crate::response::{AddressMap, AddressParams};
use hyper::Uri;

// Three map errors

pub fn create_address_map(config: &Config) -> Result<AddressMap, String> {
    let mut hashmap = AddressMap::new();
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

fn add_addresses_to_map(
    url_map: &mut AddressMap,
    addresses: &Vec<(String, String)>,
    is_dangerous: bool,
) -> Result<(), String> {
    for (source_str, target_str) in addresses {
        let source_uri = match Uri::try_from(source_str) {
            Ok(uri) => uri,
            Err(e) => return Err(e.to_string()),
        };

        let source_host = match source_uri.host() {
            Some(h) => h,
            _ => return Err("could not parse host from source uri".to_string()),
        };

        let uri = match Uri::try_from(target_str) {
            Ok(uri) => uri,
            Err(e) => return Err(e.to_string()),
        };

        url_map.insert(source_host.to_string(), AddressParams { uri, is_dangerous });
    }

    Ok(())
}
