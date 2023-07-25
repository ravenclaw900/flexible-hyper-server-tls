use flexible_hyper_server_tls::*;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use tokio::net::TcpListener;

const CERT_DATA: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/cert.pem"));
const KEY_DATA: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/key.pem"));

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
        let tls_acceptor = tlsconfig::get_tlsacceptor_from_pem_data(
            CERT_DATA,
            KEY_DATA,
            &tlsconfig::HttpProtocol::Both,
        )
        .unwrap();
        HyperHttpOrHttpsAcceptor::new_https(listener, tls_acceptor)
    } else {
        HyperHttpOrHttpsAcceptor::new_http(listener)
    };

    let mut server = Server::builder(acceptor).serve(make_svc);

    loop {
        let res = (&mut server).await;
        eprintln!("Error: {:?}", res);
    }
}
