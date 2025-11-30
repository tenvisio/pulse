//! Internal message types for Pulse.
//!
//! These types are used internally for routing and communication.

use bytes::Bytes;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// A unique message identifier.
pub type MessageId = u64;

/// Atomic counter for ensuring unique IDs even within the same nanosecond.
static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique message ID.
#[must_use]
pub fn generate_message_id() -> MessageId {
    // Combine timestamp with atomic counter for guaranteed uniqueness
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    // Use lower bits for counter, upper bits for timestamp
    timestamp.wrapping_add(counter)
}

/// An internal message for routing.
#[derive(Debug, Clone)]
pub struct Message {
    /// Unique message identifier.
    pub id: MessageId,
    /// Source connection ID.
    pub source: Option<String>,
    /// Target channel.
    pub channel: String,
    /// Optional event name.
    pub event: Option<String>,
    /// Message payload (shared for zero-copy broadcast).
    pub payload: Arc<Bytes>,
    /// Timestamp when the message was created.
    pub timestamp: u64,
}

impl Message {
    /// Create a new message.
    #[must_use]
    pub fn new(channel: impl Into<String>, payload: impl Into<Bytes>) -> Self {
        Self {
            id: generate_message_id(),
            source: None,
            channel: channel.into(),
            event: None,
            payload: Arc::new(payload.into()),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    /// Create a message with a source connection.
    #[must_use]
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Create a message with an event name.
    #[must_use]
    pub fn with_event(mut self, event: impl Into<String>) -> Self {
        self.event = Some(event.into());
        self
    }

    /// Get the payload bytes.
    #[must_use]
    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

    /// Get the payload size in bytes.
    #[must_use]
    pub fn payload_size(&self) -> usize {
        self.payload.len()
    }
}

/// A message ready for delivery to a connection.
#[derive(Debug, Clone)]
pub struct DeliveryMessage {
    /// The message to deliver.
    pub message: Message,
    /// Target connection ID.
    pub target: String,
}

impl DeliveryMessage {
    /// Create a new delivery message.
    #[must_use]
    pub fn new(message: Message, target: impl Into<String>) -> Self {
        Self {
            message,
            target: target.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::new("test-channel", b"hello".to_vec());
        assert_eq!(msg.channel, "test-channel");
        assert_eq!(&msg.payload[..], b"hello");
        assert!(msg.source.is_none());
    }

    #[test]
    fn test_message_with_source() {
        let msg = Message::new("test", b"data".to_vec())
            .with_source("conn-123")
            .with_event("user:message");

        assert_eq!(msg.source, Some("conn-123".to_string()));
        assert_eq!(msg.event, Some("user:message".to_string()));
    }

    #[test]
    fn test_unique_message_ids() {
        let id1 = generate_message_id();
        let id2 = generate_message_id();
        // IDs should be different (with high probability)
        assert_ne!(id1, id2);
    }
}
