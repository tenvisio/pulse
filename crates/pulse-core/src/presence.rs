//! Presence tracking for Pulse.
//!
//! Presence allows tracking which users are online in a channel
//! and sharing metadata about them.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::debug;

/// Presence state for a single user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceState {
    /// Connection ID.
    pub connection_id: String,
    /// User-defined metadata.
    pub data: Option<serde_json::Value>,
    /// When the user joined.
    pub joined_at: u64,
    /// Last activity timestamp.
    pub last_seen: u64,
}

impl PresenceState {
    /// Create a new presence state.
    #[must_use]
    pub fn new(connection_id: impl Into<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            connection_id: connection_id.into(),
            data: None,
            joined_at: now,
            last_seen: now,
        }
    }

    /// Create a presence state with metadata.
    #[must_use]
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Update the last seen timestamp.
    pub fn touch(&mut self) {
        self.last_seen = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }

    /// Update the metadata.
    pub fn update_data(&mut self, data: serde_json::Value) {
        self.data = Some(data);
        self.touch();
    }

    /// Check if this presence is stale (no activity for the given duration).
    #[must_use]
    pub fn is_stale(&self, timeout: Duration) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let timeout_ms = timeout.as_millis() as u64;
        now - self.last_seen > timeout_ms
    }
}

/// Presence tracker for a channel.
#[derive(Debug, Default)]
pub struct Presence {
    /// Map of connection ID to presence state.
    members: HashMap<String, PresenceState>,
}

impl Presence {
    /// Create a new presence tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the number of present members.
    #[must_use]
    pub fn count(&self) -> usize {
        self.members.len()
    }

    /// Check if a connection is present.
    #[must_use]
    pub fn is_present(&self, connection_id: &str) -> bool {
        self.members.contains_key(connection_id)
    }

    /// Get the presence state for a connection.
    #[must_use]
    pub fn get(&self, connection_id: &str) -> Option<&PresenceState> {
        self.members.get(connection_id)
    }

    /// Add a member to presence.
    ///
    /// Returns `true` if this is a new member, `false` if updating existing.
    pub fn join(
        &mut self,
        connection_id: impl Into<String>,
        data: Option<serde_json::Value>,
    ) -> bool {
        let conn_id = connection_id.into();
        let is_new = !self.members.contains_key(&conn_id);

        let mut state = PresenceState::new(conn_id.clone());
        if let Some(d) = data {
            state = state.with_data(d);
        }

        self.members.insert(conn_id.clone(), state);

        if is_new {
            debug!(connection = %conn_id, "Presence: member joined");
        }

        is_new
    }

    /// Remove a member from presence.
    ///
    /// Returns the removed presence state, if any.
    pub fn leave(&mut self, connection_id: &str) -> Option<PresenceState> {
        let state = self.members.remove(connection_id);
        if state.is_some() {
            debug!(connection = %connection_id, "Presence: member left");
        }
        state
    }

    /// Update a member's presence data.
    ///
    /// Returns `true` if the member exists and was updated.
    pub fn update(&mut self, connection_id: &str, data: serde_json::Value) -> bool {
        if let Some(state) = self.members.get_mut(connection_id) {
            state.update_data(data);
            true
        } else {
            false
        }
    }

    /// Touch a member's last seen timestamp.
    pub fn touch(&mut self, connection_id: &str) {
        if let Some(state) = self.members.get_mut(connection_id) {
            state.touch();
        }
    }

    /// Get all present members.
    #[must_use]
    pub fn members(&self) -> Vec<&PresenceState> {
        self.members.values().collect()
    }

    /// Get all connection IDs.
    #[must_use]
    pub fn connection_ids(&self) -> Vec<&str> {
        self.members.keys().map(|s| s.as_str()).collect()
    }

    /// Remove stale members (no activity for the given duration).
    ///
    /// Returns the list of removed connection IDs.
    pub fn prune_stale(&mut self, timeout: Duration) -> Vec<String> {
        let stale: Vec<String> = self
            .members
            .iter()
            .filter(|(_, state)| state.is_stale(timeout))
            .map(|(id, _)| id.clone())
            .collect();

        for id in &stale {
            self.members.remove(id);
            debug!(connection = %id, "Presence: pruned stale member");
        }

        stale
    }

    /// Get full presence state as a serializable snapshot.
    #[must_use]
    pub fn snapshot(&self) -> Vec<PresenceState> {
        self.members.values().cloned().collect()
    }

    /// Check if presence is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_presence_state() {
        let state = PresenceState::new("conn-1").with_data(json!({"name": "Alice"}));

        assert_eq!(state.connection_id, "conn-1");
        assert!(state.data.is_some());
    }

    #[test]
    fn test_presence_join_leave() {
        let mut presence = Presence::new();

        assert!(presence.join("conn-1", None));
        assert!(!presence.join("conn-1", None)); // Already present

        assert_eq!(presence.count(), 1);
        assert!(presence.is_present("conn-1"));

        assert!(presence.leave("conn-1").is_some());
        assert!(!presence.is_present("conn-1"));
    }

    #[test]
    fn test_presence_update() {
        let mut presence = Presence::new();
        presence.join("conn-1", None);

        assert!(presence.update("conn-1", json!({"status": "away"})));
        assert!(!presence.update("conn-2", json!({}))); // Doesn't exist

        let state = presence.get("conn-1").unwrap();
        assert!(state.data.is_some());
    }

    #[test]
    fn test_presence_snapshot() {
        let mut presence = Presence::new();
        presence.join("conn-1", Some(json!({"name": "Alice"})));
        presence.join("conn-2", Some(json!({"name": "Bob"})));

        let snapshot = presence.snapshot();
        assert_eq!(snapshot.len(), 2);
    }
}
