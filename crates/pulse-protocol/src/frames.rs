//! Frame types for the Pulse protocol.
//!
//! Frames are the fundamental unit of communication in Pulse.
//! Each frame is serialized using MessagePack for efficient binary encoding.

use serde::{Deserialize, Serialize};

/// Frame type identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "u8", try_from = "u8")]
#[repr(u8)]
pub enum FrameType {
    Subscribe = 0x01,
    Unsubscribe = 0x02,
    Publish = 0x03,
    Presence = 0x04,
    Ack = 0x05,
    Error = 0x06,
    Ping = 0x07,
    Pong = 0x08,
    Connect = 0x09,
    Connected = 0x0A,
}

impl From<FrameType> for u8 {
    fn from(ft: FrameType) -> u8 {
        ft as u8
    }
}

impl TryFrom<u8> for FrameType {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, <Self as TryFrom<u8>>::Error> {
        match value {
            0x01 => Ok(FrameType::Subscribe),
            0x02 => Ok(FrameType::Unsubscribe),
            0x03 => Ok(FrameType::Publish),
            0x04 => Ok(FrameType::Presence),
            0x05 => Ok(FrameType::Ack),
            0x06 => Ok(FrameType::Error),
            0x07 => Ok(FrameType::Ping),
            0x08 => Ok(FrameType::Pong),
            0x09 => Ok(FrameType::Connect),
            0x0A => Ok(FrameType::Connected),
            _ => Err("Invalid frame type"),
        }
    }
}

/// Presence action types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "u8", try_from = "u8")]
#[repr(u8)]
pub enum PresenceAction {
    /// Client joined the channel.
    Join = 0,
    /// Client left the channel.
    Leave = 1,
    /// Client updated their presence data.
    Update = 2,
    /// Server sending full presence state sync.
    Sync = 3,
}

impl From<PresenceAction> for u8 {
    fn from(pa: PresenceAction) -> u8 {
        pa as u8
    }
}

impl TryFrom<u8> for PresenceAction {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PresenceAction::Join),
            1 => Ok(PresenceAction::Leave),
            2 => Ok(PresenceAction::Update),
            3 => Ok(PresenceAction::Sync),
            _ => Err("Invalid presence action"),
        }
    }
}

/// A protocol frame.
///
/// Frames are the messages exchanged between clients and servers.
/// Each frame type has specific fields relevant to its operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Frame {
    /// Subscribe to a channel.
    #[serde(rename = "subscribe")]
    Subscribe {
        /// Request ID for acknowledgment.
        id: u64,
        /// Channel name to subscribe to.
        channel: String,
    },

    /// Unsubscribe from a channel.
    #[serde(rename = "unsubscribe")]
    Unsubscribe {
        /// Request ID for acknowledgment.
        id: u64,
        /// Channel name to unsubscribe from.
        channel: String,
    },

    /// Publish a message to a channel.
    #[serde(rename = "publish")]
    Publish {
        /// Optional request ID for acknowledgment.
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<u64>,
        /// Target channel.
        channel: String,
        /// Optional event name.
        #[serde(skip_serializing_if = "Option::is_none")]
        event: Option<String>,
        /// Message payload.
        #[serde(with = "serde_bytes")]
        payload: Vec<u8>,
    },

    /// Presence update.
    #[serde(rename = "presence")]
    Presence {
        /// Request ID.
        id: u64,
        /// Channel name.
        channel: String,
        /// Presence action.
        action: PresenceAction,
        /// Optional presence metadata.
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
    },

    /// Acknowledgment of a request.
    #[serde(rename = "ack")]
    Ack {
        /// ID of the acknowledged request.
        id: u64,
    },

    /// Error response.
    #[serde(rename = "error")]
    Error {
        /// ID of the failed request (0 if not applicable).
        id: u64,
        /// Error code.
        code: u16,
        /// Human-readable error message.
        message: String,
    },

    /// Keepalive ping.
    #[serde(rename = "ping")]
    Ping {
        /// Optional timestamp.
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<u64>,
    },

    /// Keepalive pong.
    #[serde(rename = "pong")]
    Pong {
        /// Echoed timestamp from ping.
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<u64>,
    },

    /// Initial connection handshake.
    #[serde(rename = "connect")]
    Connect {
        /// Protocol version.
        version: u8,
        /// Optional authentication token.
        #[serde(skip_serializing_if = "Option::is_none")]
        token: Option<String>,
    },

    /// Connection established response.
    #[serde(rename = "connected")]
    Connected {
        /// Unique connection identifier.
        connection_id: String,
        /// Negotiated protocol version.
        version: u8,
        /// Recommended heartbeat interval in milliseconds.
        heartbeat: u32,
    },
}

impl Frame {
    /// Get the frame type.
    #[must_use]
    pub fn frame_type(&self) -> FrameType {
        match self {
            Frame::Subscribe { .. } => FrameType::Subscribe,
            Frame::Unsubscribe { .. } => FrameType::Unsubscribe,
            Frame::Publish { .. } => FrameType::Publish,
            Frame::Presence { .. } => FrameType::Presence,
            Frame::Ack { .. } => FrameType::Ack,
            Frame::Error { .. } => FrameType::Error,
            Frame::Ping { .. } => FrameType::Ping,
            Frame::Pong { .. } => FrameType::Pong,
            Frame::Connect { .. } => FrameType::Connect,
            Frame::Connected { .. } => FrameType::Connected,
        }
    }

    /// Create a new Subscribe frame.
    #[must_use]
    pub fn subscribe(id: u64, channel: impl Into<String>) -> Self {
        Frame::Subscribe {
            id,
            channel: channel.into(),
        }
    }

    /// Create a new Unsubscribe frame.
    #[must_use]
    pub fn unsubscribe(id: u64, channel: impl Into<String>) -> Self {
        Frame::Unsubscribe {
            id,
            channel: channel.into(),
        }
    }

    /// Create a new Publish frame.
    #[must_use]
    pub fn publish(channel: impl Into<String>, payload: impl Into<Vec<u8>>) -> Self {
        Frame::Publish {
            id: None,
            channel: channel.into(),
            event: None,
            payload: payload.into(),
        }
    }

    /// Create a new Publish frame with ID for acknowledgment.
    #[must_use]
    pub fn publish_with_ack(
        id: u64,
        channel: impl Into<String>,
        payload: impl Into<Vec<u8>>,
    ) -> Self {
        Frame::Publish {
            id: Some(id),
            channel: channel.into(),
            event: None,
            payload: payload.into(),
        }
    }

    /// Create a new Ack frame.
    #[must_use]
    pub fn ack(id: u64) -> Self {
        Frame::Ack { id }
    }

    /// Create a new Error frame.
    #[must_use]
    pub fn error(id: u64, code: u16, message: impl Into<String>) -> Self {
        Frame::Error {
            id,
            code,
            message: message.into(),
        }
    }

    /// Create a new Ping frame.
    #[must_use]
    pub fn ping() -> Self {
        Frame::Ping { timestamp: None }
    }

    /// Create a new Ping frame with timestamp.
    #[must_use]
    pub fn ping_with_timestamp(timestamp: u64) -> Self {
        Frame::Ping {
            timestamp: Some(timestamp),
        }
    }

    /// Create a new Pong frame.
    #[must_use]
    pub fn pong(timestamp: Option<u64>) -> Self {
        Frame::Pong { timestamp }
    }

    /// Create a new Connect frame.
    #[must_use]
    pub fn connect(version: u8, token: Option<String>) -> Self {
        Frame::Connect { version, token }
    }

    /// Create a new Connected frame.
    #[must_use]
    pub fn connected(connection_id: impl Into<String>, version: u8, heartbeat: u32) -> Self {
        Frame::Connected {
            connection_id: connection_id.into(),
            version,
            heartbeat,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_type() {
        let subscribe = Frame::subscribe(1, "test");
        assert_eq!(subscribe.frame_type(), FrameType::Subscribe);

        let publish = Frame::publish("test", b"hello".to_vec());
        assert_eq!(publish.frame_type(), FrameType::Publish);
    }

    #[test]
    fn test_presence_action_conversion() {
        assert_eq!(PresenceAction::try_from(0), Ok(PresenceAction::Join));
        assert_eq!(PresenceAction::try_from(1), Ok(PresenceAction::Leave));
        assert_eq!(PresenceAction::try_from(2), Ok(PresenceAction::Update));
        assert_eq!(PresenceAction::try_from(3), Ok(PresenceAction::Sync));
        assert!(PresenceAction::try_from(4).is_err());
    }
}
