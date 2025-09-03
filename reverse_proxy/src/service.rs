use hyper::body::Incoming;
use hyper::service::Service;
use hyper::{Request, Uri};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use response;

pub struct Svc {
    pub addresses: Arc<HashMap<String, (Uri, bool)>>,
}

impl Service<Request<Incoming>> for Svc {
    type Response = response::BoxedResponse;
    type Error = hyper::http::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let addresses = self.addresses.clone();

        Box::pin(async move { response::build_response(req, addresses).await })
    }
}
