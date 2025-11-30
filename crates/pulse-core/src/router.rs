//! High-performance message router for Pulse.
//!
//! The router manages channels and handles pub/sub message routing.

use crate::channel::{validate_channel_name, Channel, ChannelId};
use crate::message::Message;
use crate::presence::{Presence, PresenceState};
use dashmap::DashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::broadcast;
use tracing::{debug, info, trace, warn};

/// Router errors.
#[derive(Debug, Error)]
pub enum RouterError {
    /// Invalid channel name.
    #[error("Invalid channel name: {0}")]
    InvalidChannel(&'static str),

    /// Channel not found.
    #[error("Channel not found: {0}")]
    ChannelNotFound(String),

    /// Not subscribed to channel.
    #[error("Not subscribed to channel: {0}")]
    NotSubscribed(String),

    /// Already subscribed to channel.
    #[error("Already subscribed to channel: {0}")]
    AlreadySubscribed(String),

    /// Maximum subscriptions reached.
    #[error("Maximum subscriptions reached")]
    MaxSubscriptionsReached,

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Router configuration.
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Maximum number of channels.
    pub max_channels: usize,
    /// Maximum subscriptions per connection.
    pub max_subscriptions_per_connection: usize,
    /// Channel broadcast capacity.
    pub channel_capacity: usize,
    /// Whether to auto-create channels on subscribe.
    pub auto_create_channels: bool,
    /// Whether to auto-delete empty channels.
    pub auto_delete_empty_channels: bool,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            max_channels: 10_000,
            max_subscriptions_per_connection: 100,
            channel_capacity: 1024,
            auto_create_channels: true,
            auto_delete_empty_channels: true,
        }
    }
}

/// Channel entry with presence tracking.
struct ChannelEntry {
    channel: Channel,
    presence: Presence,
}

impl ChannelEntry {
    fn new(name: impl Into<ChannelId>, capacity: usize) -> Self {
        Self {
            channel: Channel::with_capacity(name, capacity),
            presence: Presence::new(),
        }
    }
}

/// The central message router.
///
/// The router manages all channels and handles message routing between
/// publishers and subscribers using lock-free data structures.
pub struct Router {
    /// Channels indexed by name.
    channels: DashMap<ChannelId, ChannelEntry>,
    /// Connection subscriptions (connection_id -> set of channel names).
    subscriptions: DashMap<String, dashmap::DashSet<ChannelId>>,
    /// Configuration.
    config: RouterConfig,
}

impl Router {
    /// Create a new router with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(RouterConfig::default())
    }

    /// Create a new router with custom configuration.
    #[must_use]
    pub fn with_config(config: RouterConfig) -> Self {
        info!("Creating router with config: {:?}", config);
        Self {
            channels: DashMap::new(),
            subscriptions: DashMap::new(),
            config,
        }
    }

    /// Get router statistics.
    #[must_use]
    pub fn stats(&self) -> RouterStats {
        RouterStats {
            channel_count: self.channels.len(),
            connection_count: self.subscriptions.len(),
            total_subscriptions: self.subscriptions.iter().map(|s| s.len()).sum(),
        }
    }

    /// Subscribe a connection to a channel.
    ///
    /// Returns a receiver for messages on the channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the channel name is invalid or limits are exceeded.
    pub fn subscribe(
        &self,
        connection_id: &str,
        channel_name: &str,
    ) -> Result<broadcast::Receiver<Arc<Message>>, RouterError> {
        // Validate channel name
        validate_channel_name(channel_name).map_err(RouterError::InvalidChannel)?;

        // Check subscription limits
        let conn_subs = self
            .subscriptions
            .entry(connection_id.to_string())
            .or_default();

        if conn_subs.len() >= self.config.max_subscriptions_per_connection {
            return Err(RouterError::MaxSubscriptionsReached);
        }

        if conn_subs.contains(channel_name) {
            return Err(RouterError::AlreadySubscribed(channel_name.to_string()));
        }

        // Get or create channel
        let mut entry = self
            .channels
            .entry(channel_name.to_string())
            .or_insert_with(|| {
                debug!(channel = %channel_name, "Creating new channel");
                ChannelEntry::new(channel_name, self.config.channel_capacity)
            });

        // Subscribe
        let receiver = entry.channel.subscribe(connection_id);
        conn_subs.insert(channel_name.to_string());

        debug!(
            channel = %channel_name,
            connection = %connection_id,
            subscribers = entry.channel.subscriber_count(),
            "Subscribed"
        );

        Ok(receiver)
    }

    /// Unsubscribe a connection from a channel.
    ///
    /// # Errors
    ///
    /// Returns an error if not subscribed.
    pub fn unsubscribe(&self, connection_id: &str, channel_name: &str) -> Result<(), RouterError> {
        // Remove from connection's subscriptions
        if let Some(conn_subs) = self.subscriptions.get(connection_id) {
            if conn_subs.remove(channel_name).is_none() {
                return Err(RouterError::NotSubscribed(channel_name.to_string()));
            }
        } else {
            return Err(RouterError::NotSubscribed(channel_name.to_string()));
        }

        // Remove from channel
        if let Some(mut entry) = self.channels.get_mut(channel_name) {
            entry.channel.unsubscribe(connection_id);
            entry.presence.leave(connection_id);

            debug!(
                channel = %channel_name,
                connection = %connection_id,
                subscribers = entry.channel.subscriber_count(),
                "Unsubscribed"
            );

            // Auto-delete empty channels
            if self.config.auto_delete_empty_channels && entry.channel.is_empty() {
                drop(entry); // Release the lock
                self.channels.remove(channel_name);
                debug!(channel = %channel_name, "Deleted empty channel");
            }
        }

        Ok(())
    }

    /// Unsubscribe a connection from all channels.
    pub fn unsubscribe_all(&self, connection_id: &str) {
        if let Some((_, channels)) = self.subscriptions.remove(connection_id) {
            for channel_name in channels.iter() {
                if let Some(mut entry) = self.channels.get_mut(channel_name.as_str()) {
                    entry.channel.unsubscribe(connection_id);
                    entry.presence.leave(connection_id);

                    if self.config.auto_delete_empty_channels && entry.channel.is_empty() {
                        let name = channel_name.clone();
                        drop(entry);
                        self.channels.remove(&name);
                    }
                }
            }
        }

        debug!(connection = %connection_id, "Unsubscribed from all channels");
    }

    /// Publish a message to a channel.
    ///
    /// Returns the number of subscribers that received the message.
    pub fn publish(&self, message: Message) -> usize {
        let channel_name = message.channel.clone();

        if let Some(entry) = self.channels.get(&channel_name) {
            let count = entry.channel.publish(message);
            trace!(channel = %channel_name, recipients = count, "Published message");
            count
        } else {
            warn!(channel = %channel_name, "Publish to non-existent channel");
            0
        }
    }

    /// Publish raw payload to a channel.
    pub fn publish_to(&self, channel_name: &str, payload: impl Into<bytes::Bytes>) -> usize {
        let message = Message::new(channel_name, payload);
        self.publish(message)
    }

    /// Check if a channel exists.
    #[must_use]
    pub fn channel_exists(&self, channel_name: &str) -> bool {
        self.channels.contains_key(channel_name)
    }

    /// Get the subscriber count for a channel.
    #[must_use]
    pub fn subscriber_count(&self, channel_name: &str) -> usize {
        self.channels
            .get(channel_name)
            .map(|e| e.channel.subscriber_count())
            .unwrap_or(0)
    }

    /// Get all channel names.
    #[must_use]
    pub fn channel_names(&self) -> Vec<String> {
        self.channels.iter().map(|e| e.key().clone()).collect()
    }

    /// Join presence for a channel.
    pub fn presence_join(
        &self,
        connection_id: &str,
        channel_name: &str,
        data: Option<serde_json::Value>,
    ) -> bool {
        if let Some(mut entry) = self.channels.get_mut(channel_name) {
            entry.presence.join(connection_id, data)
        } else {
            false
        }
    }

    /// Leave presence for a channel.
    pub fn presence_leave(&self, connection_id: &str, channel_name: &str) -> Option<PresenceState> {
        if let Some(mut entry) = self.channels.get_mut(channel_name) {
            entry.presence.leave(connection_id)
        } else {
            None
        }
    }

    /// Get presence snapshot for a channel.
    #[must_use]
    pub fn presence_snapshot(&self, channel_name: &str) -> Vec<PresenceState> {
        self.channels
            .get(channel_name)
            .map(|e| e.presence.snapshot())
            .unwrap_or_default()
    }

    /// Get the channels a connection is subscribed to.
    #[must_use]
    pub fn connection_channels(&self, connection_id: &str) -> Vec<String> {
        self.subscriptions
            .get(connection_id)
            .map(|s| s.iter().map(|c| c.clone()).collect())
            .unwrap_or_default()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Router statistics.
#[derive(Debug, Clone)]
pub struct RouterStats {
    /// Number of active channels.
    pub channel_count: usize,
    /// Number of connected clients.
    pub connection_count: usize,
    /// Total number of subscriptions.
    pub total_subscriptions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_subscribe_unsubscribe() {
        let router = Router::new();

        // Subscribe
        let rx = router.subscribe("conn-1", "test:channel").unwrap();
        assert!(router.channel_exists("test:channel"));
        assert_eq!(router.subscriber_count("test:channel"), 1);
        drop(rx);

        // Unsubscribe
        router.unsubscribe("conn-1", "test:channel").unwrap();
        // Channel should be auto-deleted
        assert!(!router.channel_exists("test:channel"));
    }

    #[test]
    fn test_router_publish() {
        let router = Router::new();

        let mut rx1 = router.subscribe("conn-1", "test").unwrap();
        let mut rx2 = router.subscribe("conn-2", "test").unwrap();

        let count = router.publish_to("test", b"hello".to_vec());
        assert_eq!(count, 2);

        // Both should receive
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    #[test]
    fn test_router_invalid_channel() {
        let router = Router::new();

        assert!(router.subscribe("conn-1", "").is_err());
        assert!(router.subscribe("conn-1", "$system").is_err());
    }

    #[test]
    fn test_router_already_subscribed() {
        let router = Router::new();

        let _rx = router.subscribe("conn-1", "test").unwrap();
        assert!(matches!(
            router.subscribe("conn-1", "test"),
            Err(RouterError::AlreadySubscribed(_))
        ));
    }

    #[test]
    fn test_router_unsubscribe_all() {
        let router = Router::new();

        let _rx1 = router.subscribe("conn-1", "channel-1").unwrap();
        let _rx2 = router.subscribe("conn-1", "channel-2").unwrap();

        router.unsubscribe_all("conn-1");

        assert!(!router.channel_exists("channel-1"));
        assert!(!router.channel_exists("channel-2"));
    }

    #[test]
    fn test_router_stats() {
        let router = Router::new();

        let _rx1 = router.subscribe("conn-1", "channel-1").unwrap();
        let _rx2 = router.subscribe("conn-1", "channel-2").unwrap();
        let _rx3 = router.subscribe("conn-2", "channel-1").unwrap();

        let stats = router.stats();
        assert_eq!(stats.channel_count, 2);
        assert_eq!(stats.connection_count, 2);
        assert_eq!(stats.total_subscriptions, 3);
    }
}
