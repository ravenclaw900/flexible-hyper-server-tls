use std::time::Duration;

use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::accept::HttpOrHttpsAcceptor;

pub struct Http;
pub struct Https {
    tls_acceptor: tokio_rustls::TlsAcceptor,
    max_handshakes: usize,
    timeout: Duration,
}

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
                max_handshakes: tls_listener::DEFAULT_MAX_HANDSHAKES,
                timeout: tls_listener::DEFAULT_HANDSHAKE_TIMEOUT,
            },
            listener: self.listener,
        }
    }

    /// Builds an `HttpOrHttpsAcceptor` to accept HTTP connections
    pub fn build(self) -> HttpOrHttpsAcceptor {
        HttpOrHttpsAcceptor::Http(self.listener)
    }
}

impl AcceptorBuilder<Https> {
    /// Set the maximum number of handshakes that will be processed concurrently
    ///
    /// Defaults to 64
    #[must_use]
    pub const fn max_handshakes(mut self, num: usize) -> Self {
        self.state.max_handshakes = num;
        self
    }

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

        tls_builder
            .max_handshakes(self.state.max_handshakes)
            .handshake_timeout(self.state.timeout);

        let tls_listener = tls_builder.listen(self.listener);

        HttpOrHttpsAcceptor::Https(tls_listener)
    }
}
