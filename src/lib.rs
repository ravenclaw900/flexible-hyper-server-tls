#![warn(clippy::pedantic, clippy::nursery, rust_2018_idioms)]
#![allow(clippy::module_name_repetitions)]

//! This library lets you easily create a Hyper acceptor that be configured to either accept HTTP or HTTPS connections.
//! This is useful for applications that users will self-host, and have the option to run as HTTP or provide their own HTTPS certificates.
//! **Note: HTTP and HTTPS cannot be accepted at the same time, you decide which one to use when creating the acceptor.**
//! ## Example
//! ```
//! use flexible_hyper_server_tls::*;
//! use hyper::service::{make_service_fn, service_fn};
//! use hyper::{Body, Request, Response, Server};
//! use std::convert::Infallible;
//! use tokio::net::TcpListener;
//!
//! async fn hello_world(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
//!     Ok(Response::new("Hello, World".into()))
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let use_tls = true;
//!
//!     let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
//!
//!     let make_svc = make_service_fn(|conn: &HttpOrHttpsConnection| {
//!         println!("Remote address: {}", conn.remote_addr());
//!         async { Ok::<_, Infallible>(service_fn(hello_world)) }
//!     });
//!
//!     let acceptor = if use_tls {
//!         let tls_acceptor = tlsconfig::get_tlsacceptor_from_files(
//!             "./cert.cer",
//!             "./key.pem",
//!             &tlsconfig::HttpProtocol::Both,
//!         )
//!         .unwrap();
//!         HyperHttpOrHttpsAcceptor::new_https(listener, tls_acceptor)
//!     } else {
//!         HyperHttpOrHttpsAcceptor::new_http(listener)
//!     };
//!
//!     let server = Server::builder(acceptor).serve(make_svc);
//!
//!     server.await.unwrap();
//! }
//! ```

mod accept;
mod conn;
pub mod tlsconfig;

// Export into main library
pub use accept::HyperHttpOrHttpsAcceptor;
pub use conn::HttpOrHttpsConnection;
