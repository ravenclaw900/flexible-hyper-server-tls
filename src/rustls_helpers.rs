//! Provides a couple of functions that assist in getting a `TlsAcceptor` from certificate and key data.
//!
//! These functions use safe defaults from rustls to generate the `TlsAcceptor`, but it is not necessary to use them.

use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tokio_rustls::rustls;

/// Error when creating a `TlsAcceptor`
#[derive(Error, Debug)]
pub enum TlsAcceptorError {
    /// PEM data was invalid
    #[error("invalid pem data")]
    InvalidPem(#[source] std::io::Error),
    /// Rustls failed to create the `ServerConfig`
    #[error("failed to create ServerConfig")]
    ServerConfig(#[from] rustls::Error),
    /// Failed to read a file
    #[error("failed to read file")]
    FileRead(#[source] std::io::Error),
}

// Only HTTP/1 is supported at the moment

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

/// Get a `TlsAcceptor` from PEM-encoded certificate and key files
///
/// # Errors
/// Errors if the files cannot be read, if there is no valid certificate/key data given, or if rustls fails to create
/// the server config
pub async fn get_tlsacceptor_from_files(
    cert_path: impl AsRef<Path> + Send,
    key_path: impl AsRef<Path> + Send,
) -> Result<tokio_rustls::TlsAcceptor, TlsAcceptorError> {
    let cert_data = tokio::fs::read(cert_path)
        .await
        .map_err(TlsAcceptorError::FileRead)?;
    let key_data = tokio::fs::read(key_path)
        .await
        .map_err(TlsAcceptorError::FileRead)?;

    get_tlsacceptor_from_pem_data(&cert_data, &key_data)
}

/// Get a `TlsAcceptor` from PEM certificate and key data
///
/// # Errors
/// Errors if there is no valid certificate/key data given or if rustls fails to create the server config
pub fn get_tlsacceptor_from_pem_data(
    mut cert_data: &[u8],
    mut key_data: &[u8],
) -> Result<tokio_rustls::TlsAcceptor, TlsAcceptorError> {
    let certs: Vec<_> = rustls_pemfile::certs(&mut cert_data)
        .collect::<Result<_, _>>()
        .map_err(TlsAcceptorError::InvalidPem)?;

    let key = rustls_pemfile::private_key(&mut key_data)
        .map_err(TlsAcceptorError::InvalidPem)?
        .ok_or_else(|| {
            TlsAcceptorError::InvalidPem(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "missing private key",
            ))
        })?;

    let mut cfg = rustls::server::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    cfg.alpn_protocols = vec![b"http/1.1".to_vec(), b"http/1.0".to_vec()];

    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(cfg));

    Ok(acceptor)
}
