# Flexible Hyper Server TLS

A library that lets you choose whether to accept HTTPS or HTTP connections with Hyper. Very useful for situations where applications are self-hosted and the user gets to optionally provide their own HTTPS certificates.

This library also provides some helper functions that simplify the TLS setup by using the safe defaults from Rustls.

The aim of this library is to be simple and have minimal extra dependencies, while still allowing the user to customize things like TLS config.

For situations where you don't need to choose between HTTP and HTTPS, check out [simple-hyper-server-tls](https://crates.io/crates/simple-hyper-server-tls) or [tls-listener](https://crates.io/crates/tls-listener).

## Usage
```rust
use flexible_hyper_server_tls::*;
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::service::service_fn;
use hyper::{Request, Response};
use std::convert::Infallible;
use tokio::net::TcpListener;

async fn hello_world(_req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::<Bytes>::from("Hello, World!")))
}

#[tokio::main]
async fn main() {
    let use_tls = true;

    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    let mut acceptor = if use_tls {
        let tls = rustls_helpers::get_tlsacceptor_from_files("./cert.cer", "./key.pem").unwrap();
        HttpOrHttpsAcceptor::new_https(listener, tls_acceptor)
    } else {
        HttpOrHttpsAcceptor::new_http(listener)
    };

    acceptor.serve(service_fn(hello_world), |err| {
        eprintln!("Error serving connection: {err:?}")
    }).await;
}
```
