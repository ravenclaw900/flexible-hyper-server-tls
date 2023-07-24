use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};

/// The stream connecting to a client over HTTP or HTTPS
/// Yielded by `HyperHttpOrHttpsAcceptor`
pub struct HttpOrHttpsConnection {
    pub(crate) remote_addr: std::net::SocketAddr,
    pub(crate) kind: ConnKind,
}

pub enum ConnKind {
    Http(tokio::net::TcpStream),
    Https(tokio_rustls::server::TlsStream<tokio::net::TcpStream>),
}

impl HttpOrHttpsConnection {
    /// Get the remote `SocketAddr` of the connected client
    pub const fn remote_addr(&self) -> std::net::SocketAddr {
        self.remote_addr
    }
}

impl AsyncRead for HttpOrHttpsConnection {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match &mut self.kind {
            ConnKind::Http(tcp) => Pin::new(tcp).poll_read(cx, buf),
            ConnKind::Https(tls) => Pin::new(tls).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for HttpOrHttpsConnection {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        match &mut self.kind {
            ConnKind::Http(tcp) => Pin::new(tcp).poll_write(cx, buf),
            ConnKind::Https(tls) => Pin::new(tls).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        match &mut self.kind {
            ConnKind::Http(tcp) => Pin::new(tcp).poll_flush(cx),
            ConnKind::Https(tls) => Pin::new(tls).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        match &mut self.kind {
            ConnKind::Http(tcp) => Pin::new(tcp).poll_shutdown(cx),
            ConnKind::Https(tls) => Pin::new(tls).poll_shutdown(cx),
        }
    }
}
