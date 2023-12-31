use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use thiserror::Error;

/// Choose to accept either a HTTP or HTTPS connection
///
/// Created by calling the `build` method on an `AcceptorBuilder`
// Use a struct instead of the enum directly to avoid users constructing/matching on enum variants
pub struct HttpOrHttpsAcceptor(pub(crate) AcceptorInner);

pub enum AcceptorInner {
    Http(tokio::net::TcpListener),
    Https(tls_listener::TlsListener<tokio::net::TcpListener, tokio_rustls::TlsAcceptor>),
}

impl HttpOrHttpsAcceptor {
    /// Accepts every connection using the service provided, never completes.
    /// Ignores any connection errors produced by the `accept` method.
    pub async fn serve<S>(&mut self, service: S)
    where
        S: hyper::service::HttpService<hyper::body::Incoming> + Clone + Send + 'static,
        S::Future: Send,
        S::ResBody: Send + 'static,
        <S::ResBody as hyper::body::Body>::Error: std::error::Error + Send + Sync + 'static,
        <S::ResBody as hyper::body::Body>::Data: Send,
    {
        loop {
            // Ignore result here
            let _ = self.accept(service.clone()).await;
        }
    }

    /// Accepts a singular connection and spawns it onto the tokio runtime.
    /// Returns the address of the connected client.
    ///
    /// # Errors
    /// If the TCP connection or TLS handshake (HTTPS only) fails
    // Function won't panic, however tokio worker might
    #[allow(clippy::missing_panics_doc)]
    pub async fn accept<S>(&mut self, service: S) -> Result<SocketAddr, AcceptorError>
    where
        S: hyper::service::HttpService<hyper::body::Incoming> + Send + 'static,
        S::Future: Send,
        S::ResBody: Send + 'static,
        <S::ResBody as hyper::body::Body>::Error: std::error::Error + Send + Sync + 'static,
        <S::ResBody as hyper::body::Body>::Data: Send,
    {
        let conn_builder = http1::Builder::new();

        match &mut self.0 {
            AcceptorInner::Http(listener) => {
                let (conn, peer_addr) =
                    listener.accept().await.map_err(AcceptorError::TcpConnect)?;

                let conn = TokioIo::new(conn);

                let conn = conn_builder.serve_connection(conn, service);

                tokio::spawn(async move { conn.await.unwrap() });

                Ok(peer_addr)
            }
            AcceptorInner::Https(listener) => {
                let (conn, peer_addr) = loop {
                    match listener.accept().await {
                        Err(tls_listener::Error::ListenerError(e)) => {
                            return Err(AcceptorError::TcpConnect(e))
                        }
                        Err(tls_listener::Error::TlsAcceptError { error, .. }) => {
                            return Err(AcceptorError::TcpConnect(error))
                        }
                        // Ignore handshake timeout errors, just try to get another connection
                        Err(_) => continue,
                        Ok(conn_and_addr) => break conn_and_addr,
                    }
                };

                let conn = TokioIo::new(conn);

                let conn = conn_builder.serve_connection(conn, service);

                tokio::spawn(async move { conn.await.unwrap() });

                Ok(peer_addr)
            }
        }
    }
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
}
