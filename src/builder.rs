use std::time::Duration;

use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::accept::{AcceptorInner, HttpOrHttpsAcceptor};

pub struct Http;
pub struct Https {
    tls_acceptor: tokio_rustls::TlsAcceptor,
    timeout: Duration,
}

/// Build an `HttpOrHttpsAcceptor`
///
/// Defaults to accepting HTTP connections, call the `https` method to accept HTTPS connections instead
pub struct AcceptorBuilder<State> {
    state: State,
    listener: TcpListener,
}

impl AcceptorBuilder<Http> {
    /// Create a new builder for an `HttpOrHttpsAcceptor`
    ///
    /// Defaults to accepting HTTP
    pub const fn new(listener: TcpListener) -> Self {
        Self {
            state: Http,
            listener,
        }
    }

    /// Converts the builder into accepting HTTPS using the provided `TlsAcceptor`
    pub fn https(self, tls_acceptor: TlsAcceptor) -> AcceptorBuilder<Https> {
        AcceptorBuilder {
            state: Https {
                tls_acceptor,
                timeout: tls_listener::DEFAULT_HANDSHAKE_TIMEOUT,
            },
            listener: self.listener,
        }
    }

    /// Builds an `HttpOrHttpsAcceptor` to accept HTTP connections
    pub fn build(self) -> HttpOrHttpsAcceptor {
        HttpOrHttpsAcceptor(AcceptorInner::Http(self.listener))
    }
}

impl AcceptorBuilder<Https> {
    /// Set the maximum amount of time that a handshake can take before being aborted.
    /// Setting it to 0 will not disable the timeout, but will instead instantly drop every connection.
    ///
    /// Defaults to 10 seconds
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.state.timeout = timeout;
        self
    }

    /// Builds an `HttpOrHttpsAcceptor` to accept HTTPS connections
    pub fn build(self) -> HttpOrHttpsAcceptor {
        let mut tls_builder = tls_listener::builder(self.state.tls_acceptor);

        tls_builder.handshake_timeout(self.state.timeout);

        let tls_listener = tls_builder.listen(self.listener);

        HttpOrHttpsAcceptor(AcceptorInner::Https(tls_listener))
    }
}
