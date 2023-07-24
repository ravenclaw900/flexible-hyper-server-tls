#![warn(clippy::pedantic, clippy::nursery, rust_2018_idioms)]
#![allow(clippy::module_name_repetitions)]

//! This library lets you easily create a Hyper acceptor that be configured to either accept HTTP or HTTPS connections.
//! This is useful for applications that users will self-host, and have the option to run as HTTP or provide their own HTTPS certificates.
//! **Note: HTTP and HTTPS cannot be accepted at the same time, you decide which one to use when creating the acceptor.**
//! ## Example
//! TBD

mod accept;
mod conn;

// Export into main library
pub use accept::HyperHttpOrHttpsAcceptor;
pub use conn::HttpOrHttpsConnection;
