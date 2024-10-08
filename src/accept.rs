use hyper::body::{Body, Incoming};
use hyper::server::conn::http1;
use hyper::service::HttpService;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use thiserror::Error;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;

use crate::stream::HttpOrHttpsStream;

/// Accept either an HTTP or HTTPS connection using Hyper
pub struct HttpOrHttpsAcceptor {
    pub(crate) listener: TcpListener,
    pub(crate) tls: Option<TlsAcceptor>,
}

impl HttpOrHttpsAcceptor {
    /// Create a new `HttpOrHttpsAcceptor` to serve HTTP.
    pub const fn new_http(listener: TcpListener) -> Self {
        Self {
            listener,
            tls: None,
        }
    }

    /// Create a new `HttpOrHttpsAcceptor` to serve HTTPS with a provided [`TlsAcceptor`].
    pub const fn new_https(listener: TcpListener, tls: TlsAcceptor) -> Self {
        Self {
            listener,
            tls: Some(tls),
        }
    }

    /// Accepts every connection using the service provided, never completes.
    ///
    /// # Errors
    /// Never returns an error, but `err_handler` will be called if the TCP connection, TLS handshake, or Hyper connection fails.
    pub async fn serve<S, F>(&mut self, service: S, err_handler: F)
    where
        S: hyper::service::HttpService<hyper::body::Incoming> + Clone + Send + Sync + 'static,
        S::Future: Send,
        S::ResBody: Send,
        <S::ResBody as Body>::Error: std::error::Error + Send + Sync + 'static,
        <S::ResBody as Body>::Data: Send,
        F: FnOnce(AcceptorError) + Clone + Send + Sync + 'static,
    {
        loop {
            if let Err(err) = self.accept(service.clone(), err_handler.clone()).await {
                (err_handler.clone())(err);
            }
        }
    }

    /// Accepts a singular connection and spawns it onto the tokio runtime.
    /// Returns the address of the connected client.
    ///
    /// # Errors
    /// Errors if the TCP connection fails. `err_handler` will be called if the TLS handshake or Hyper connection fails.
    #[allow(clippy::missing_panics_doc)]
    pub async fn accept<S, F>(
        &mut self,
        service: S,
        err_handler: F,
    ) -> Result<SocketAddr, AcceptorError>
    where
        S: HttpService<Incoming> + Send + 'static,
        S::Future: Send,
        S::ResBody: Send + 'static,
        <S::ResBody as Body>::Error: std::error::Error + Send + Sync,
        <S::ResBody as Body>::Data: Send,
        F: FnOnce(AcceptorError) + Send + 'static,
    {
        let (stream, peer_addr) = self
            .listener
            .accept()
            .await
            .map_err(AcceptorError::TcpConnect)?;

        // The TlsAcceptor is a wrapper around an Arc, so this is relatively cheap
        let tls_clone = self.tls.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_conn(stream, tls_clone, service).await {
                err_handler(err);
            }
        });

        Ok(peer_addr)
    }
}

async fn handle_conn<S>(
    stream: TcpStream,
    tls: Option<TlsAcceptor>,
    handler: S,
) -> Result<(), AcceptorError>
where
    S: HttpService<Incoming> + Send,
    S::Future: Send,
    S::ResBody: Send + 'static,
    <S::ResBody as Body>::Error: std::error::Error + Send + Sync,
    <S::ResBody as Body>::Data: Send,
{
    let client = match tls {
        None => HttpOrHttpsStream::Http(stream),
        Some(tls) => {
            let tls_stream = tls
                .accept(stream)
                .await
                .map_err(AcceptorError::TlsHandshake)?;
            HttpOrHttpsStream::Https(tls_stream)
        }
    };

    // Use `with_upgrades` to allow usage of websockets in client code
    http1::Builder::new()
        .serve_connection(TokioIo::new(client), handler)
        .with_upgrades()
        .await
        .map_err(AcceptorError::Hyper)
}

/// Error when accepting connections
#[derive(Error, Debug)]
pub enum AcceptorError {
    /// Failed to connect to client over TCP
    #[error("TCP connection to client failed")]
    TcpConnect(#[source] std::io::Error),
    /// Failed to make TLS handshake with client
    #[error("TLS handshake with client failed")]
    TlsHandshake(#[source] std::io::Error),
    /// Hyper failed to serve connection
    #[error("Failed to serve HTTP connection")]
    Hyper(#[source] hyper::Error),
}
