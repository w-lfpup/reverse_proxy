use hyper::body::Incoming;
use hyper::service::Service;
use hyper::{Request, StatusCode};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::addresses;
use crate::requests;

const URI_FROM_REQUEST_ERROR: &str = "unable to parse upstream URI from request";
const UPSTREAM_URI_ERROR: &str = "falied to update request with upstream URI";

pub struct Svc {
    pub addresses: Arc<HashMap<String, (http::Uri, bool)>>,
}

impl Service<Request<Incoming>> for Svc {
    type Response = requests::BoxedResponse;
    type Error = hyper::http::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, mut req: Request<Incoming>) -> Self::Future {
        // get origin host from request
        let host = match addresses::get_host(&req) {
            Some(uri) => uri,
            _ => {
                return Box::pin(async {
                    // bad request
                    requests::create_fallback_response(
                        &StatusCode::BAD_REQUEST,
                        &URI_FROM_REQUEST_ERROR,
                    )
                });
            }
        };

        // get target uri
        let (target_uri, is_dangerous) = match self.addresses.get(&host) {
            Some((trgt_uri, is_dngrs)) => (trgt_uri.clone(), is_dngrs.clone()),
            _ => {
                return Box::pin(async {
                    // bad request
                    requests::create_fallback_response(
                        &StatusCode::BAD_GATEWAY,
                        &URI_FROM_REQUEST_ERROR,
                    )
                });
            }
        };

        // replace dest_uri path and query with target path and query
        if let Err(_) = update_request_with_dest_uri(&mut req, target_uri) {
            return Box::pin(async {
                requests::create_fallback_response(
                    &StatusCode::INTERNAL_SERVER_ERROR,
                    &UPSTREAM_URI_ERROR,
                )
            });
        };

        return Box::pin(async move { requests::get_response(req, is_dangerous).await });
    }
}

fn update_request_with_dest_uri(
    req: &mut Request<Incoming>,
    target_uri: http::Uri,
) -> Result<(), String> {
    let target_path_opt = req.uri().path_and_query();
    let mut dest_parts = target_uri.into_parts();

    // start with nothing
    dest_parts.path_and_query = None;
    if let Some(path_and_query) = target_path_opt {
        dest_parts.path_and_query = Some(path_and_query.clone());
    }

    // dest_parts.path_and_query = target_path_opt.clone();
    if let None = dest_parts.scheme {
        dest_parts.scheme = Some(http::uri::Scheme::HTTP);
    }

    *req.uri_mut() = match http::Uri::from_parts(dest_parts) {
        Ok(u) => u,
        Err(e) => return Err(e.to_string()),
    };

    Ok(())
}
