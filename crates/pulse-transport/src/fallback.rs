//! Fallback transport negotiation.
//!
//! This module provides automatic transport selection and fallback
//! when the preferred transport is unavailable.

use crate::traits::{Connection, Transport, TransportError};
use async_trait::async_trait;
use std::sync::Arc;

/// A transport that tries multiple transports in order of preference.
pub struct FallbackTransport {
    transports: Vec<Arc<dyn Transport>>,
}

impl FallbackTransport {
    /// Create a new fallback transport with the given transports.
    ///
    /// Transports are tried in order (first = highest priority).
    #[must_use]
    pub fn new(transports: Vec<Arc<dyn Transport>>) -> Self {
        Self { transports }
    }

    /// Add a transport to the fallback chain.
    pub fn add_transport(&mut self, transport: Arc<dyn Transport>) {
        self.transports.push(transport);
    }

    /// Get the list of transport names in priority order.
    #[must_use]
    pub fn transport_names(&self) -> Vec<&'static str> {
        self.transports.iter().map(|t| t.name()).collect()
    }
}

#[async_trait]
impl Transport for FallbackTransport {
    async fn accept(&self) -> Result<Box<dyn Connection>, TransportError> {
        // In a real implementation, this would use select! to accept
        // from multiple transports concurrently
        for transport in &self.transports {
            if transport.is_healthy() {
                return transport.accept().await;
            }
        }

        Err(TransportError::Other(
            "No healthy transports available".into(),
        ))
    }

    fn name(&self) -> &'static str {
        "fallback"
    }

    fn is_healthy(&self) -> bool {
        self.transports.iter().any(|t| t.is_healthy())
    }
}

/// Negotiate the best transport for a client.
///
/// This function examines the client's capabilities and selects
/// the most appropriate transport.
#[must_use]
pub fn negotiate_transport(
    client_capabilities: &[&str],
    available_transports: &[&str],
) -> Option<&'static str> {
    // Priority order: WebTransport > WebSocket > SSE
    let priority = ["webtransport", "websocket", "sse"];

    for transport in priority {
        if client_capabilities.contains(&transport) && available_transports.contains(&transport) {
            // Return static str for the matched transport
            return match transport {
                "webtransport" => Some("webtransport"),
                "websocket" => Some("websocket"),
                "sse" => Some("sse"),
                _ => None,
            };
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negotiate_transport() {
        // Client supports both, server has both
        assert_eq!(
            negotiate_transport(
                &["websocket", "webtransport"],
                &["websocket", "webtransport"]
            ),
            Some("webtransport")
        );

        // Client only supports websocket
        assert_eq!(
            negotiate_transport(&["websocket"], &["websocket", "webtransport"]),
            Some("websocket")
        );

        // No common transport
        assert_eq!(negotiate_transport(&["sse"], &["websocket"]), None);
    }
}
