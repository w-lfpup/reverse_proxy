use hyper::body::Incoming;
use hyper::service::Service;
use hyper::Request;
use response::{build_response, AddressMap};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub struct Svc {
    pub addresses: Arc<AddressMap>,
}

impl Service<Request<Incoming>> for Svc {
    type Response = response::BoxedResponse;
    type Error = hyper::http::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let addresses = self.addresses.clone();

        Box::pin(async move { build_response(req, addresses).await })
    }
}
