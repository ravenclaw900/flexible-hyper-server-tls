//! Provides a couple of functions that assist in getting a `TlsAcceptor` from certificate and key data.
//!
//! These functions use safe defaults from rustls to generate the `TlsAcceptor`, but it is not necessary to use them.

use std::fs::File;
use std::io::{BufRead, BufReader, Cursor};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tokio_rustls::rustls;

/// Error when creating a `TlsAcceptor`
#[derive(Error, Debug)]
pub enum TlsAcceptorError {
    /// No valid PEM data was provided
    #[error("no valid pem data")]
    NoValidPem,
    /// There were no private keys in the provided PEM data
    #[error("no valid private keys in pem data")]
    NoValidKey,
    /// Rustls failed to create the `ServerConfig`
    #[error("failed to create ServerConfig")]
    ServerConfig(#[from] rustls::Error),
    /// Failed to open a PEM file
    #[error("failed to open pem file")]
    FileOpen(#[source] std::io::Error),
    /// General IO errors
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// Only HTTP 1 is supported at the moment

// /// The HTTP protocol to use when clients are connecting.
// ///
// /// This should match the version(s) of HTTP used to serve your application in Hyper.
// /// Using `Both` will prefer HTTP/2 over HTTP/1.1
// #[derive(Debug, Clone, Copy)]
// pub enum HttpProtocol {
//     Http1,
//     Http2,
//     Both,
// }

/// Get a `TlsAcceptor` from PEM certificate and key data
///
/// # Errors
/// Errors if there is no valid certificate/key data given, or if rustls fails to create
/// the server config
pub fn get_tlsacceptor_from_pem_data(
    cert_data: &str,
    key_data: &str,
) -> Result<tokio_rustls::TlsAcceptor, TlsAcceptorError> {
    let mut cert_reader = BufReader::new(Cursor::new(cert_data));
    let mut key_reader = BufReader::new(Cursor::new(key_data));
    get_tlsacceptor_from_readers(&mut cert_reader, &mut key_reader)
}

/// Get a `TlsAcceptor` from PEM-encoded certificate and key files
///
/// # Errors
/// Errors if the files cannot be read, if there is no valid certificate/key data given, or if rustls fails to create
/// the server config
pub fn get_tlsacceptor_from_files(
    cert_path: impl AsRef<Path>,
    key_path: impl AsRef<Path>,
) -> Result<tokio_rustls::TlsAcceptor, TlsAcceptorError> {
    let cert_file = File::open(cert_path).map_err(TlsAcceptorError::FileOpen)?;
    let key_file = File::open(key_path).map_err(TlsAcceptorError::FileOpen)?;

    let mut cert_reader = BufReader::new(cert_file);
    let mut key_reader = BufReader::new(key_file);

    get_tlsacceptor_from_readers(&mut cert_reader, &mut key_reader)
}

fn get_tlsacceptor_from_readers(
    cert_reader: &mut dyn BufRead,
    key_reader: &mut dyn BufRead,
) -> Result<tokio_rustls::TlsAcceptor, TlsAcceptorError> {
    let certs: Vec<_> = rustls_pemfile::certs(cert_reader)
        .filter_map(Result::ok)
        .collect();

    let key = rustls_pemfile::read_one(key_reader)?.ok_or(TlsAcceptorError::NoValidPem)?;
    let key = match key {
        rustls_pemfile::Item::Sec1Key(data) => data.into(),
        rustls_pemfile::Item::Pkcs1Key(data) => data.into(),
        rustls_pemfile::Item::Pkcs8Key(data) => data.into(),
        _ => return Err(TlsAcceptorError::NoValidKey),
    };

    let mut cfg = rustls::server::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    cfg.alpn_protocols = vec![b"http/1.1".to_vec(), b"http/1.0".to_vec()];

    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(cfg));

    Ok(acceptor)
}
