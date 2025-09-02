use http::Uri;
use hyper::body::Incoming;
use hyper::service::Service;
use hyper::Request;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::requests;

pub struct Svc {
    pub addresses: Arc<HashMap<String, (Uri, bool)>>,
}

impl Service<Request<Incoming>> for Svc {
    type Response = requests::BoxedResponse;
    type Error = hyper::http::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let addresses = self.addresses.clone();

        Box::pin(async move { requests::create_response(req, addresses).await })
    }
}
