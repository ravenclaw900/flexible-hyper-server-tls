use std::net::SocketAddr;
use std::sync::Arc;

use hyper::body::{Body, Incoming};
use hyper::server::conn::http1;
use hyper::service::HttpService;
use hyper_util::rt::TokioIo;
use thiserror::Error;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;

use crate::stream::HttpOrHttpsStream;

/// Accept either an HTTP or HTTPS connection using Hyper
pub struct HttpOrHttpsAcceptor {
    listener: TcpListener,
    tls: Option<TlsAcceptor>,
    err_handler: Arc<dyn Fn(AcceptorError) + Send + Sync>,
}

impl HttpOrHttpsAcceptor {
    /// Creates a new [`HttpOrHttpsAcceptor`] with the default configuration (serve HTTP, silently ignore errors)
    pub fn new(listener: TcpListener) -> Self {
        Self {
            listener,
            tls: None,
            err_handler: Arc::new(|_| {}),
        }
    }

    /// Configures this [`HttpOrHttpsAcceptor`] to serve HTTPS using the provided [`TlsAcceptor`]
    ///
    /// If you need to create a [`TlsAcceptor`], see the helper functions in [`rustls_helpers`](crate::rustls_helpers)
    #[must_use]
    pub fn with_tls(mut self, tls: TlsAcceptor) -> Self {
        self.tls = Some(tls);
        self
    }

    /// Configures this [`HttpOrHttpsAcceptor`] to call the provided error handler on errors
    #[must_use]
    pub fn with_err_handler<F>(mut self, err_handler: F) -> Self
    where
        F: Fn(AcceptorError) + Send + Sync + 'static,
    {
        self.err_handler = Arc::new(err_handler);
        self
    }

    /// Accepts a singular connection and spawns it onto the tokio runtime.
    /// Returns the address of the connected client.
    ///
    /// # Errors
    /// Never returns an error. The configured error handler will be called if the TCP connection, TLS handshake, or Hyper connection fails.
    pub async fn accept<S>(&mut self, service: S) -> SocketAddr
    where
        S: HttpService<Incoming> + Send + 'static,
        S::Future: Send,
        S::ResBody: Send,
        <S::ResBody as Body>::Error: std::error::Error + Send + Sync,
        <S::ResBody as Body>::Data: Send,
    {
        loop {
            match self.listener.accept().await {
                Ok((stream, peer_addr)) => {
                    // The TlsAcceptor is a wrapper around an Arc, so this is relatively cheap
                    let cloned_tls = self.tls.clone();
                    let cloned_err_handler = self.err_handler.clone();

                    tokio::spawn(async move {
                        if let Err(err) = handle_conn(stream, cloned_tls, service).await {
                            cloned_err_handler(err);
                        }
                    });

                    return peer_addr;
                }
                Err(e) => (self.err_handler)(AcceptorError::TcpConnect(e)),
            };
        }
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
