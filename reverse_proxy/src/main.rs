use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use native_tls::Identity;
use std::sync::Arc;
use std::{env, path};
use tokio::fs;
use tokio::net::TcpListener;

mod addresses;
mod config;
mod service;

// Needs to be errors
// create key errors

#[tokio::main]
async fn main() -> Result<(), String> {
    // create config
    let args = match env::args().nth(1) {
        Some(a) => path::PathBuf::from(a),
        None => return Err("argument error: argv[0] config path not provided".to_string()),
    };
    let config = match config::from_filepath(&args).await {
        Ok(c) => c,
        Err(e) => return Err(e),
    };

    // if destination URIs fail to parse, the server fails to run.
    let addresses = match addresses::create_address_map(&config) {
        Ok(addrs) => Arc::new(addrs),
        Err(e) => return Err(e),
    };

    // tls cert and keys
    let cert = match fs::read(&config.cert_filepath).await {
        Ok(f) => f,
        Err(e) => return Err(e.to_string()),
    };
    let key = match fs::read(&config.key_filepath).await {
        Ok(f) => f,
        Err(e) => return Err(e.to_string()),
    };
    let identity = match Identity::from_pkcs8(&cert, &key) {
        Ok(pk) => pk,
        Err(e) => return Err(e.to_string()),
    };

    // create tls acceptor
    let tls_acceptor = match native_tls::TlsAcceptor::new(identity) {
        Ok(acceptor) => tokio_native_tls::TlsAcceptor::from(acceptor),
        Err(e) => return Err(e.to_string()),
    };

    // bind tcp listeners
    let listener = match TcpListener::bind(&config.host_and_port).await {
        Ok(l) => l,
        Err(e) => return Err(e.to_string()),
    };

    println!("Reverse Proxy: {}", &config.host_and_port);

    loop {
        let (socket, _remote_addr) = match listener.accept().await {
            Ok(s) => s,
            Err(e) => return Err(e.to_string()),
        };

        let acceptor = tls_acceptor.clone();

        let service = service::Svc {
            addresses: addresses.clone(),
        };

        tokio::task::spawn(async move {
            let io = match acceptor.accept(socket).await {
                Ok(s) => TokioIo::new(s),
                Err(_e) => return,
            };

            if let Err(e) = Builder::new(TokioExecutor::new())
                .serve_connection(io, service)
                .await
            {
                println!("{}", e);
                return;
            }
        });
    }
}
