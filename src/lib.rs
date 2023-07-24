#![warn(clippy::pedantic, clippy::nursery, rust_2018_idioms)]
#![allow(clippy::module_name_repetitions)]

mod accept;
mod conn;

// Export into main library
pub use accept::HyperHttpOrHttpsAcceptor;
pub use conn::HttpOrHttpsConnection;
