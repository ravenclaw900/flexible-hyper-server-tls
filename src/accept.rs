use futures_util::future::BoxFuture;
use futures_util::stream::FuturesUnordered;
use futures_util::{FutureExt, StreamExt};
use hyper::server::accept::Accept;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::conn::{ConnKind, HttpOrHttpsConnection};

/// Choose to accept either a HTTP or HTTPS connection
pub struct HyperHttpOrHttpsAcceptor {
    listener: tokio::net::TcpListener,
    kind: AcceptorKind,
}

enum AcceptorKind {
    Http,
    Https {
        tls_acceptor: tokio_rustls::TlsAcceptor,
        timeout: std::time::Duration,
        // Future has to be boxed because Rust doesn't allow writing out the full type
        // Side benefit of allow us to use Timeout without needing pin projection
        encryption_futures: FuturesUnordered<
            tokio::time::Timeout<BoxFuture<'static, Result<HttpOrHttpsConnection, std::io::Error>>>,
        >,
    },
}

impl HyperHttpOrHttpsAcceptor {
    /// Create an acceptor that will accept HTTP connections
    pub const fn new_http(listener: tokio::net::TcpListener) -> Self {
        Self {
            listener,
            kind: AcceptorKind::Http,
        }
    }

    /// Create an acceptor that will accept HTTPS connections using the provided `TlsAcceptor`
    ///
    /// `handshake_timeout` is the length of time that should be allowed to finish a TLS handshake before we drop the connection.
    /// Setting it to 0 will not disable the timeout, but will instead instantly drop every connection (you probably don't want this).
    pub fn new_https(
        listener: tokio::net::TcpListener,
        tls_acceptor: tokio_rustls::TlsAcceptor,
        handshake_timeout: std::time::Duration,
    ) -> Self {
        Self {
            listener,
            kind: AcceptorKind::Https {
                tls_acceptor,
                timeout: handshake_timeout,
                encryption_futures: FuturesUnordered::new(),
            },
        }
    }
}

impl Accept for HyperHttpOrHttpsAcceptor {
    type Conn = HttpOrHttpsConnection;
    type Error = std::io::Error;

    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        // Necessary to allow partial borrows
        let this = self.get_mut();

        match &mut this.kind {
            // If just a normal HTTP connection, just poll to accept the new TCP connection
            AcceptorKind::Http => match this.listener.poll_accept(cx) {
                Poll::Ready(Ok(stream)) => Poll::Ready(Some(Ok(HttpOrHttpsConnection {
                    remote_addr: stream.1,
                    kind: ConnKind::Http(stream.0),
                }))),
                Poll::Ready(Err(err)) => Poll::Ready(Some(Err(err))),
                Poll::Pending => Poll::Pending,
            },
            // Otherwise, if it's an HTTPS connection, check if we're ready to encrypt the connection
            AcceptorKind::Https {
                tls_acceptor,
                timeout,
                encryption_futures,
            } => {
                // Accept all pending TCP connections at once (this future won't be woken up for TCP unless we get a pending here)
                loop {
                    match this.listener.poll_accept(cx) {
                        Poll::Ready(Ok(stream)) => {
                            let tls_future = tls_acceptor
                                .accept(stream.0)
                                .map(move |f| {
                                    f.map(|conn| HttpOrHttpsConnection {
                                        remote_addr: stream.1,
                                        kind: ConnKind::Https(conn),
                                    })
                                })
                                .boxed();
                            let timed_tls_future = tokio::time::timeout(*timeout, tls_future);
                            encryption_futures.push(timed_tls_future);
                        }
                        Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err))),
                        // Break on pending here so we can check on the TLS queue
                        Poll::Pending => break,
                    }
                }
                // Check queue to see if any handshakes are done/timeouts hit
                loop {
                    match encryption_futures.poll_next_unpin(cx) {
                        // Already `map`ed to a Result<HttpOrHttpsConnection>, so no need to differentiate
                        // between Some(Err) and Some(Ok)
                        Poll::Ready(Some(Ok(res))) => return Poll::Ready(Some(res)),
                        // An error here means that the timeout ran out, so just skip to the next one in the queue
                        Poll::Ready(Some(Err(_))) => continue,
                        _ => return Poll::Pending,
                    }
                }
            }
        }
    }
}
