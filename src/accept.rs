use hyper::server::accept::Accept;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::conn::{ConnKind, HttpOrHttpsConnection};

pub struct HyperHttpOrHttpsAcceptor {
    listener: tokio::net::TcpListener,
    kind: AcceptorKind,
}

enum AcceptorKind {
    Http,
    Https {
        tls_acceptor: tokio_rustls::TlsAcceptor,
        encryption_future: Option<HttpsEncryptionFuture>,
    },
}

struct HttpsEncryptionFuture {
    future: tokio_rustls::Accept<tokio::net::TcpStream>,
    // Only the HTTPS side needs to store the remote address, HTTP can just return it immediately
    remote_addr: std::net::SocketAddr,
}

impl HyperHttpOrHttpsAcceptor {
    pub const fn new_http(listener: tokio::net::TcpListener) -> Self {
        Self {
            listener,
            kind: AcceptorKind::Http,
        }
    }

    pub const fn new_https(
        listener: tokio::net::TcpListener,
        tls_acceptor: tokio_rustls::TlsAcceptor,
    ) -> Self {
        Self {
            listener,
            kind: AcceptorKind::Https {
                tls_acceptor,
                encryption_future: None,
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
            // Weird control flow here (if then another if) to avoid returning an unnecessary Poll::Pending
            AcceptorKind::Https {
                tls_acceptor,
                encryption_future,
            } => {
                // Will be skipped if going through a second time, as accept_future will have already been stored
                if encryption_future.is_none() {
                    match this.listener.poll_accept(cx) {
                        Poll::Ready(Ok(stream)) => {
                            *encryption_future = Some(HttpsEncryptionFuture {
                                future: tls_acceptor.accept(stream.0),
                                remote_addr: stream.1,
                            });
                        }
                        Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err))),
                        Poll::Pending => return Poll::Pending,
                    }
                }
                // Unwrap is safe because encryption_future has to have been created by now
                let encryption_future = encryption_future.as_mut().unwrap();
                match Pin::new(&mut encryption_future.future).poll(cx) {
                    Poll::Ready(Ok(tls_stream)) => Poll::Ready(Some(Ok(HttpOrHttpsConnection {
                        remote_addr: encryption_future.remote_addr,
                        kind: ConnKind::Https(tls_stream),
                    }))),
                    Poll::Ready(Err(err)) => Poll::Ready(Some(Err(err))),
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}
