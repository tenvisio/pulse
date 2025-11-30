//! Channel abstraction for Pulse.
//!
//! Channels are named rooms where connections can subscribe to receive messages.

use crate::message::Message;
use bytes::Bytes;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, trace};

/// Maximum channel name length.
pub const MAX_CHANNEL_NAME_LENGTH: usize = 256;

/// Default broadcast channel capacity.
const DEFAULT_CHANNEL_CAPACITY: usize = 1024;

/// A channel identifier.
pub type ChannelId = String;

/// Validate a channel name.
///
/// # Errors
///
/// Returns an error message if the channel name is invalid.
pub fn validate_channel_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("Channel name cannot be empty");
    }
    if name.len() > MAX_CHANNEL_NAME_LENGTH {
        return Err("Channel name too long");
    }
    if name.starts_with('$') {
        return Err("Channel names starting with '$' are reserved");
    }
    // Check for valid ASCII printable characters
    if !name.chars().all(|c| c.is_ascii() && !c.is_ascii_control()) {
        return Err("Channel name contains invalid characters");
    }
    Ok(())
}

/// A channel for pub/sub messaging.
#[derive(Debug)]
pub struct Channel {
    /// Channel name.
    name: ChannelId,
    /// Broadcast sender for this channel.
    sender: broadcast::Sender<Arc<Message>>,
    /// Set of subscribed connection IDs.
    subscribers: HashSet<String>,
    /// Channel capacity.
    capacity: usize,
}

impl Channel {
    /// Create a new channel.
    #[must_use]
    pub fn new(name: impl Into<ChannelId>) -> Self {
        Self::with_capacity(name, DEFAULT_CHANNEL_CAPACITY)
    }

    /// Create a new channel with a specific capacity.
    #[must_use]
    pub fn with_capacity(name: impl Into<ChannelId>, capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            name: name.into(),
            sender,
            subscribers: HashSet::new(),
            capacity,
        }
    }

    /// Get the channel name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the number of subscribers.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    /// Check if a connection is subscribed.
    #[must_use]
    pub fn is_subscribed(&self, connection_id: &str) -> bool {
        self.subscribers.contains(connection_id)
    }

    /// Subscribe a connection to this channel.
    ///
    /// Returns a receiver for messages on this channel.
    pub fn subscribe(
        &mut self,
        connection_id: impl Into<String>,
    ) -> broadcast::Receiver<Arc<Message>> {
        let conn_id = connection_id.into();
        self.subscribers.insert(conn_id.clone());
        debug!(channel = %self.name, connection = %conn_id, "Connection subscribed");
        self.sender.subscribe()
    }

    /// Unsubscribe a connection from this channel.
    ///
    /// Returns `true` if the connection was subscribed.
    pub fn unsubscribe(&mut self, connection_id: &str) -> bool {
        let removed = self.subscribers.remove(connection_id);
        if removed {
            debug!(channel = %self.name, connection = %connection_id, "Connection unsubscribed");
        }
        removed
    }

    /// Publish a message to this channel.
    ///
    /// Returns the number of receivers that received the message.
    pub fn publish(&self, message: Message) -> usize {
        let msg = Arc::new(message);
        trace!(channel = %self.name, "Publishing message");
        self.sender.send(msg).unwrap_or_default()
    }

    /// Publish raw payload to this channel.
    ///
    /// Returns the number of receivers that received the message.
    pub fn publish_payload(&self, payload: impl Into<Bytes>) -> usize {
        let message = Message::new(self.name.clone(), payload);
        self.publish(message)
    }

    /// Get all subscriber IDs.
    #[must_use]
    pub fn subscribers(&self) -> Vec<String> {
        self.subscribers.iter().cloned().collect()
    }

    /// Check if the channel is empty (no subscribers).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.subscribers.is_empty()
    }

    /// Get the channel capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let channel = Channel::new("test:room");
        assert_eq!(channel.name(), "test:room");
        assert_eq!(channel.subscriber_count(), 0);
        assert!(channel.is_empty());
    }

    #[test]
    fn test_channel_subscribe_unsubscribe() {
        let mut channel = Channel::new("test");

        let _rx = channel.subscribe("conn-1");
        assert_eq!(channel.subscriber_count(), 1);
        assert!(channel.is_subscribed("conn-1"));

        let _rx2 = channel.subscribe("conn-2");
        assert_eq!(channel.subscriber_count(), 2);

        assert!(channel.unsubscribe("conn-1"));
        assert_eq!(channel.subscriber_count(), 1);
        assert!(!channel.is_subscribed("conn-1"));

        // Unsubscribing non-existent connection
        assert!(!channel.unsubscribe("conn-1"));
    }

    #[test]
    fn test_channel_name_validation() {
        assert!(validate_channel_name("valid:channel").is_ok());
        assert!(validate_channel_name("").is_err());
        assert!(validate_channel_name("$system").is_err());

        let long_name = "a".repeat(MAX_CHANNEL_NAME_LENGTH + 1);
        assert!(validate_channel_name(&long_name).is_err());
    }

    #[tokio::test]
    async fn test_channel_publish() {
        let mut channel = Channel::new("test");
        let mut rx = channel.subscribe("conn-1");

        let count = channel.publish_payload(b"hello".to_vec());
        assert_eq!(count, 1);

        let msg = rx.recv().await.unwrap();
        assert_eq!(&msg.payload[..], b"hello");
    }
}
