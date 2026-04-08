use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::body::Incoming;
use hyper::http::uri::InvalidUriParts;
use hyper::{header, Request, Response, StatusCode, Uri};
use std::collections::HashMap;
use std::sync::Arc;

mod requests;

use crate::requests::{
    create_fallback_response, send_http1_request, send_http1_tls_request, send_http2_request,
    send_http2_tls_request,
};

pub type BoxedResponse = Response<BoxBody<Bytes, hyper::Error>>;

const URI_FROM_REQUEST_ERROR: &str = "unable to parse upstream URI from request";
const UPSTREAM_URI_ERROR: &str = "falied to update request with upstream URI";

#[derive(Clone, Debug)]
pub struct AddressParams {
    pub uri: Uri,
    pub is_dangerous: bool,
}

pub type AddressMap = HashMap<String, AddressParams>;

pub async fn build_response(
    mut req: Request<Incoming>,
    addresses: Arc<HashMap<String, AddressParams>>,
) -> Result<BoxedResponse, hyper::http::Error> {
    let host = match get_host(&req) {
        Some(uri) => uri,
        _ => return create_fallback_response(&StatusCode::BAD_REQUEST, &URI_FROM_REQUEST_ERROR),
    };

    // get target uri
    let address_params = match addresses.get(&host) {
        Some(params) => params,
        _ => return create_fallback_response(&StatusCode::BAD_GATEWAY, &URI_FROM_REQUEST_ERROR),
    };

    if let Err(_) = update_request_with_dest_uri(&mut req, &address_params.uri) {
        return create_fallback_response(&StatusCode::INTERNAL_SERVER_ERROR, &UPSTREAM_URI_ERROR);
    };

    get_response(req, address_params.is_dangerous).await
}

fn get_host(req: &Request<Incoming>) -> Option<String> {
    // http2
    if let Some(host) = req.uri().host() {
        return Some(host.to_string());
    };

    // http 1
    let host_header = match req.headers().get(header::HOST) {
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

fn update_request_with_dest_uri(
    req: &mut Request<Incoming>,
    target_uri: &Uri,
) -> Result<(), InvalidUriParts> {
    let mut dest_parts = target_uri.clone().into_parts();

    // start with no path
    dest_parts.path_and_query = None;
    if let Some(path_and_query) = req.uri().path_and_query() {
        dest_parts.path_and_query = Some(path_and_query.clone());
    }

    *req.uri_mut() = match Uri::from_parts(dest_parts) {
        Ok(u) => u,
        Err(e) => return Err(e),
    };

    Ok(())
}

pub async fn get_response(
    req: Request<Incoming>,
    is_dangerous: bool,
) -> Result<BoxedResponse, hyper::http::Error> {
    let version = req.version();
    let scheme = match req.uri().scheme() {
        Some(a) => a.as_str(),
        _ => "http",
    };

    match (version, scheme) {
        (hyper::Version::HTTP_2, "https") => send_http2_tls_request(req, is_dangerous).await,
        (hyper::Version::HTTP_2, _) => send_http2_request(req).await,
        (_, "https") => send_http1_tls_request(req, is_dangerous).await,
        _ => send_http1_request(req).await,
    }
}
