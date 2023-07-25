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
        // Future has to be boxed because Rust won't let me write out the full type
        encryption_futures:
            FuturesUnordered<BoxFuture<'static, Result<HttpOrHttpsConnection, std::io::Error>>>,
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
    pub fn new_https(
        listener: tokio::net::TcpListener,
        tls_acceptor: tokio_rustls::TlsAcceptor,
    ) -> Self {
        Self {
            listener,
            kind: AcceptorKind::Https {
                tls_acceptor,
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
                encryption_futures,
            } => {
                // Accept all pending TCP connections at once
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
                            encryption_futures.push(tls_future);
                        }
                        Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err))),
                        // Break on pending here so we can check on the TLS queue
                        Poll::Pending => break,
                    }
                }
                // Check queue to see if any handshakes are done
                match encryption_futures.poll_next_unpin(cx) {
                    // Already `map`ed to a Result<HttpOrHttpsConnection>, so no need to differentiate
                    // between Some(Err) and Some(Ok)
                    Poll::Ready(Some(res)) => Poll::Ready(Some(res)),
                    _ => Poll::Pending,
                }
            }
        }
    }
}
