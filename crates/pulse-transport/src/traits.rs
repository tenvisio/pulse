//! Transport abstraction traits for Pulse.
//!
//! These traits define the interface that all transport implementations must provide,
//! allowing the server to be transport-agnostic.

use async_trait::async_trait;
use bytes::Bytes;
use pulse_protocol::Frame;
use std::fmt;
use thiserror::Error;

/// Unique identifier for a connection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConnectionId(pub String);

impl ConnectionId {
    /// Create a new connection ID.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a random connection ID.
    #[must_use]
    pub fn generate() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self(format!("conn_{:x}", timestamp))
    }

    /// Get the ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ConnectionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ConnectionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Transport errors.
#[derive(Debug, Error)]
pub enum TransportError {
    /// Connection was closed.
    #[error("Connection closed")]
    ConnectionClosed,

    /// Connection timed out.
    #[error("Connection timed out")]
    Timeout,

    /// Failed to send data.
    #[error("Send failed: {0}")]
    SendFailed(String),

    /// Failed to receive data.
    #[error("Receive failed: {0}")]
    ReceiveFailed(String),

    /// Protocol error.
    #[error("Protocol error: {0}")]
    Protocol(#[from] pulse_protocol::ProtocolError),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Other error.
    #[error("{0}")]
    Other(String),
}

/// A transport that can accept connections.
///
/// Transports are responsible for handling the underlying protocol
/// (WebSocket, WebTransport, etc.) and providing a uniform interface.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Accept a new connection.
    ///
    /// This method blocks until a new connection is available or an error occurs.
    async fn accept(&self) -> Result<Box<dyn Connection>, TransportError>;

    /// Get the transport name (e.g., "websocket", "webtransport").
    fn name(&self) -> &'static str;

    /// Check if the transport is healthy.
    fn is_healthy(&self) -> bool {
        true
    }
}

/// An active connection over a transport.
///
/// Connections handle the bidirectional flow of frames between
/// the server and a single client.
#[async_trait]
pub trait Connection: Send + Sync {
    /// Get the connection's unique identifier.
    fn id(&self) -> &ConnectionId;

    /// Receive the next frame from the connection.
    ///
    /// Returns `None` if the connection is closed cleanly.
    async fn recv(&mut self) -> Result<Option<Frame>, TransportError>;

    /// Send a frame to the connection.
    async fn send(&mut self, frame: Frame) -> Result<(), TransportError>;

    /// Send raw bytes to the connection.
    ///
    /// This is useful for pre-encoded frames to avoid re-encoding.
    async fn send_raw(&mut self, data: Bytes) -> Result<(), TransportError>;

    /// Close the connection gracefully.
    async fn close(&mut self) -> Result<(), TransportError>;

    /// Get the remote address of the connection, if available.
    fn remote_addr(&self) -> Option<String> {
        None
    }

    /// Check if the connection is still open.
    fn is_open(&self) -> bool;
}

/// Extension trait for connections with additional capabilities.
#[async_trait]
pub trait ConnectionExt: Connection {
    /// Send a frame and wait for acknowledgment.
    async fn send_with_ack(&mut self, frame: Frame, timeout_ms: u64) -> Result<(), TransportError>;

    /// Ping the connection and measure round-trip time.
    async fn ping(&mut self) -> Result<std::time::Duration, TransportError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_id_generation() {
        let id1 = ConnectionId::generate();
        let id2 = ConnectionId::generate();
        assert_ne!(id1, id2);
        assert!(id1.as_str().starts_with("conn_"));
    }

    #[test]
    fn test_connection_id_from_string() {
        let id: ConnectionId = "test-id".into();
        assert_eq!(id.as_str(), "test-id");
    }
}
