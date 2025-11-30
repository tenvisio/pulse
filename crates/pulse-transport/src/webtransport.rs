//! WebTransport transport implementation (stub).
//!
//! This module provides a WebTransport-based transport using wtransport.
//! WebTransport is still experimental and requires HTTP/3 support.

use async_trait::async_trait;
use bytes::Bytes;
use pulse_protocol::Frame;

use crate::traits::{Connection, ConnectionId, Transport, TransportError};

/// WebTransport configuration.
#[derive(Debug, Clone)]
pub struct WebTransportConfig {
    /// Address to bind to.
    pub bind_addr: std::net::SocketAddr,
    /// Path to TLS certificate.
    pub cert_path: String,
    /// Path to TLS key.
    pub key_path: String,
}

/// WebTransport transport (stub implementation).
///
/// Full implementation requires:
/// - TLS certificate configuration
/// - HTTP/3 server setup
/// - QUIC connection handling
pub struct WebTransportTransport {
    #[allow(dead_code)]
    config: WebTransportConfig,
}

impl WebTransportTransport {
    /// Create a new WebTransport transport.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    pub async fn new(config: WebTransportConfig) -> Result<Self, TransportError> {
        // TODO: Initialize wtransport server
        tracing::warn!("WebTransport support is experimental");
        Ok(Self { config })
    }
}

#[async_trait]
impl Transport for WebTransportTransport {
    async fn accept(&self) -> Result<Box<dyn Connection>, TransportError> {
        // TODO: Accept WebTransport connection
        Err(TransportError::Other(
            "WebTransport not fully implemented".into(),
        ))
    }

    fn name(&self) -> &'static str {
        "webtransport"
    }

    fn is_healthy(&self) -> bool {
        false // Not implemented yet
    }
}

/// A WebTransport connection (stub).
pub struct WebTransportConnection {
    id: ConnectionId,
}

impl WebTransportConnection {
    /// Create a new WebTransport connection.
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: ConnectionId::generate(),
        }
    }
}

impl Default for WebTransportConnection {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Connection for WebTransportConnection {
    fn id(&self) -> &ConnectionId {
        &self.id
    }

    async fn recv(&mut self) -> Result<Option<Frame>, TransportError> {
        Err(TransportError::Other(
            "WebTransport not fully implemented".into(),
        ))
    }

    async fn send(&mut self, _frame: Frame) -> Result<(), TransportError> {
        Err(TransportError::Other(
            "WebTransport not fully implemented".into(),
        ))
    }

    async fn send_raw(&mut self, _data: Bytes) -> Result<(), TransportError> {
        Err(TransportError::Other(
            "WebTransport not fully implemented".into(),
        ))
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        Ok(())
    }

    fn is_open(&self) -> bool {
        false
    }
}
