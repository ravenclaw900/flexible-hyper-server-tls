# Flexible Hyper Server TLS

A library that lets you choose whether to accept HTTPS or HTTP connections with Hyper. Very useful for situations where applications are self-hosted and the user gets to optionally provide their own HTTPS certificates.

This library also provides some helper functions that simplify the TLS setup by using the safe defaults from Rustls.

The aim of this library is to be simple and have minimal extra dependencies, while still allowing the user to customize things like TLS config.

For situations where you don't need to choose between HTTP and HTTPS, check out [simple-hyper-server-tls](https://crates.io/crates/simple-hyper-server-tls) or [tls-listener](https://crates.io/crates/tls-listener).

## Usage
```rust
use flexible_hyper_server_tls::*;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use tokio::net::TcpListener;
use std::time::Duration;

async fn hello_world(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new("Hello, World".into()))
}

#[tokio::main]
async fn main() {
    let use_tls = true;

    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    let make_svc = make_service_fn(|conn: &HttpOrHttpsConnection| {
        println!("Remote address: {}", conn.remote_addr());
        async { Ok::<_, Infallible>(service_fn(hello_world)) }
    });

    let acceptor = if use_tls {
        let tls_acceptor = tlsconfig::get_tlsacceptor_from_files(
            "./cert.cer",
            "./key.pem",
            tlsconfig::HttpProtocol::Http1,
        )
        .unwrap();
        HyperHttpOrHttpsAcceptor::new_https(listener, tls_acceptor, Duration::from_secs(10))
    } else {
        HyperHttpOrHttpsAcceptor::new_http(listener)
    };

    let server = Server::builder(acceptor).serve(make_svc);

    server.await.unwrap();
}
```
