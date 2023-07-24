#![warn(clippy::pedantic, clippy::nursery, rust_2018_idioms)]

mod accept;
mod conn;

// Export into main library
pub use accept::HyperHttpOrHttpsAcceptor;
pub use conn::HttpOrHttpsConnection;
