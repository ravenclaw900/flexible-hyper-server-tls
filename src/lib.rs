#![warn(clippy::pedantic, clippy::nursery, rust_2018_idioms)]
#![allow(clippy::module_name_repetitions)]

//! This library lets you easily create a Hyper acceptor that be configured to either accept HTTP or HTTPS connections.
//! This is useful for applications that users will self-host, and have the option to run as HTTP or provide their own HTTPS certificates.
//! At the moment, this library only supports accepting HTTP/1 connections
//! **Note: HTTP and HTTPS cannot be accepted at the same time, you decide which one to use when creating the acceptor.**
//! ## Example
//! ```
//! use flexible_hyper_server_tls::*;
//! use http_body_util::Full;
//! use hyper::body::{Bytes, Incoming};
//! use hyper::service::service_fn;
//! use hyper::{Request, Response};
//! use std::convert::Infallible;
//! use tokio::net::TcpListener;
//!
//! async fn hello_world(_req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
//!     Ok(Response::new(Full::<Bytes>::from("Hello, World!")))
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let use_tls = true;
//!
//!     let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
//!
//!     let builder = AcceptorBuilder::new(listener);
//!
//!     let mut acceptor = if use_tls {
//!         let tls_acceptor =
//!             rustls_helpers::get_tlsacceptor_from_files("./cert.cer", "./key.pem").unwrap();
//!         builder.https(tls_acceptor).build()
//!     } else {
//!         builder.build()
//!     };
//!
//!     acceptor.serve(service_fn(hello_world)).await;
//! }
//! ```

mod accept;
mod builder;
#[cfg(feature = "rustls_helpers")]
pub mod rustls_helpers;

// Export into main library
pub use accept::{AcceptorError, HttpOrHttpsAcceptor};
pub use builder::AcceptorBuilder;
