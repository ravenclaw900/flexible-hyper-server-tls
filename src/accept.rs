use std::net::SocketAddr;

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
}

impl HttpOrHttpsAcceptor {
    /// Creates a new [`HttpOrHttpsAcceptor`] configured to only serve HTTP
    pub const fn new(listener: TcpListener) -> Self {
        Self {
            listener,
            tls: None,
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

    /// Accepts a singular connection.
    /// Returns a the peer address of the connected client and a future that MUST be spawned to serve the connection.
    ///
    /// # Errors
    /// The function will return an error if the TCP connection fails, the returned future will return an error if the TLS handshake or Hyper service fails.
    pub async fn accept<S>(
        &self,
        service: S,
    ) -> Result<
        (
            SocketAddr,
            impl Future<Output = Result<(), AcceptorError>> + use<S>,
        ),
        AcceptorError,
    >
    where
        S: HttpService<Incoming> + 'static,
        <S::ResBody as Body>::Error: std::error::Error + Send + Sync,
    {
        match self.listener.accept().await {
            Ok((stream, peer_addr)) => {
                // The TlsAcceptor is a wrapper around an Arc, so this is relatively cheap
                let cloned_tls = self.tls.clone();

                let conn_fut = handle_conn(stream, cloned_tls, service);
                Ok((peer_addr, conn_fut))
            }
            Err(e) => Err(AcceptorError::TcpConnect(e)),
        }
    }
}

async fn handle_conn<S>(
    stream: TcpStream,
    tls: Option<TlsAcceptor>,
    handler: S,
) -> Result<(), AcceptorError>
where
    S: HttpService<Incoming>,
    S::ResBody: 'static,
    <S::ResBody as Body>::Error: std::error::Error + Send + Sync,
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
