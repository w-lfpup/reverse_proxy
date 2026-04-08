use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::client::conn::{http1, http2};
use hyper::{header, Request, Response, StatusCode, Uri};
use hyper_util::rt::{TokioExecutor, TokioIo};
use native_tls::TlsConnector;
use tokio::net::TcpStream;

pub type BoxedResponse = Response<BoxBody<Bytes, hyper::Error>>;

const UPSTREAM_HANDSHAKE_ERROR: &str = "upstream handshake failed";
const FAILED_TO_PROCESS_REQUEST_ERROR: &str = "failed to process request";

fn get_host_and_authority<'a>(uri: &Uri) -> Result<(String, String), &'a str> {
    let host = match uri.host() {
        Some(h) => h.to_string(),
        _ => return Err("failed to retrieve URI from upstream URI"),
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

    Ok((host.clone(), host + ":" + &port))
}

async fn create_tcp_stream<'a>(addr: &str) -> Result<TokioIo<TcpStream>, &'a str> {
    match TcpStream::connect(&addr).await {
        Ok(client_stream) => Ok(TokioIo::new(client_stream)),
        _ => Err("failed to establish tcp connection"),
    }
}

async fn create_tls_stream<'a>(
    host: &str,
    addr: &str,
    is_dangerous: bool,
) -> Result<TokioIo<tokio_native_tls::TlsStream<TcpStream>>, &'a str> {
    let mut builder = TlsConnector::builder();
    if is_dangerous {
        builder.danger_accept_invalid_certs(true);
    }
    let cx = match builder.build() {
        Ok(c) => c,
        _ => return Err("failed to build TLS connection"),
    };

    let tls_connector = tokio_native_tls::TlsConnector::from(cx);
    let client_stream = match TcpStream::connect(addr).await {
        Ok(s) => s,
        _ => return Err("failed to establish TCP connection"),
    };

    let tls_stream = match tls_connector.connect(host, client_stream).await {
        Ok(s) => TokioIo::new(s),
        _ => return Err("failed to establish TLS connection"),
    };

    Ok(tls_stream)
}

pub async fn send_http1_request(
    req: Request<Incoming>,
) -> Result<BoxedResponse, hyper::http::Error> {
    let (_, addr) = match get_host_and_authority(&req.uri()) {
        Ok(stream) => stream,
        Err(e) => return create_fallback_response(&StatusCode::BAD_REQUEST, e),
    };

    let io = match create_tcp_stream(&addr).await {
        Ok(stream) => stream,
        Err(e) => return create_fallback_response(&StatusCode::SERVICE_UNAVAILABLE, e),
    };

    let (mut sender, conn) = match http1::handshake(io).await {
        Ok(handshake) => handshake,
        Err(_) => {
            return create_fallback_response(
                &StatusCode::SERVICE_UNAVAILABLE,
                &UPSTREAM_HANDSHAKE_ERROR,
            )
        }
    };

    tokio::task::spawn(async move {
        if let Err(_err) = conn.await { /* log connection error */ }
    });

    if let Ok(r) = sender.send_request(req).await {
        return Ok(r.map(|b| b.boxed()));
    };

    create_fallback_response(&StatusCode::BAD_GATEWAY, &FAILED_TO_PROCESS_REQUEST_ERROR)
}

pub async fn send_http1_tls_request(
    req: Request<Incoming>,
    is_dangerous: bool,
) -> Result<BoxedResponse, hyper::http::Error> {
    let (host, addr) = match get_host_and_authority(&req.uri()) {
        Ok(stream) => stream,
        Err(e) => return create_fallback_response(&StatusCode::BAD_REQUEST, e),
    };

    let io = match create_tls_stream(&host, &addr, is_dangerous).await {
        Ok(stream) => stream,
        Err(e) => return create_fallback_response(&StatusCode::SERVICE_UNAVAILABLE, e),
    };

    let (mut sender, conn) = match http1::handshake(io).await {
        Ok(handshake) => handshake,
        Err(_) => {
            return create_fallback_response(
                &StatusCode::SERVICE_UNAVAILABLE,
                &UPSTREAM_HANDSHAKE_ERROR,
            )
        }
    };

    tokio::task::spawn(async move {
        if let Err(_err) = conn.await { /* log connection error */ }
    });

    if let Ok(r) = sender.send_request(req).await {
        return Ok(r.map(|b| b.boxed()));
    };

    create_fallback_response(&StatusCode::BAD_GATEWAY, &FAILED_TO_PROCESS_REQUEST_ERROR)
}

pub async fn send_http2_request(
    req: Request<Incoming>,
) -> Result<BoxedResponse, hyper::http::Error> {
    let (_, addr) = match get_host_and_authority(&req.uri()) {
        Ok(stream) => stream,
        Err(e) => return create_fallback_response(&StatusCode::BAD_REQUEST, e),
    };

    let io = match create_tcp_stream(&addr).await {
        Ok(stream) => stream,
        Err(e) => return create_fallback_response(&StatusCode::SERVICE_UNAVAILABLE, e),
    };

    let (mut client, client_conn) = match http2::handshake(TokioExecutor::new(), io).await {
        Ok(handshake) => handshake,
        Err(_) => {
            return create_fallback_response(
                &StatusCode::SERVICE_UNAVAILABLE,
                &UPSTREAM_HANDSHAKE_ERROR,
            )
        }
    };

    tokio::task::spawn(async move {
        if let Err(_err) = client_conn.await { /* log connection error */ }
    });

    if let Ok(res) = client.send_request(req).await {
        return Ok(res.map(|b| b.boxed()));
    };

    create_fallback_response(&StatusCode::BAD_GATEWAY, &FAILED_TO_PROCESS_REQUEST_ERROR)
}

pub async fn send_http2_tls_request(
    req: Request<Incoming>,
    is_dangerous: bool,
) -> Result<BoxedResponse, hyper::http::Error> {
    let (host, addr) = match get_host_and_authority(&req.uri()) {
        Ok(stream) => stream,
        Err(e) => return create_fallback_response(&StatusCode::BAD_REQUEST, e),
    };

    let io = match create_tls_stream(&host, &addr, is_dangerous).await {
        Ok(stream) => stream,
        Err(e) => return create_fallback_response(&StatusCode::SERVICE_UNAVAILABLE, e),
    };

    let (mut client, client_conn) = match http2::handshake(TokioExecutor::new(), io).await {
        Ok(handshake) => handshake,
        Err(_) => {
            return create_fallback_response(
                &StatusCode::SERVICE_UNAVAILABLE,
                &UPSTREAM_HANDSHAKE_ERROR,
            )
        }
    };

    tokio::task::spawn(async move {
        if let Err(_err) = client_conn.await { /* log connection error */ }
    });

    if let Ok(res) = client.send_request(req).await {
        return Ok(res.map(|b| b.boxed()));
    };

    create_fallback_response(&StatusCode::BAD_GATEWAY, &FAILED_TO_PROCESS_REQUEST_ERROR)
}

pub fn create_fallback_response(
    status_code: &StatusCode,
    body_str: &'static str,
) -> Result<BoxedResponse, hyper::http::Error> {
    Response::builder()
        .status(status_code)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(
            Full::new(Bytes::from(body_str))
                .map_err(|e| match e {})
                .boxed(),
        )
}
