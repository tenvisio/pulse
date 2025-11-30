//! # pulse-protocol
//!
//! Wire protocol definitions for the Pulse realtime communication engine.
//!
//! This crate defines the binary protocol used for communication between
//! Pulse clients and servers, including frame types, codecs, and versioning.
//!
//! ## Frame Types
//!
//! - `Subscribe` / `Unsubscribe` - Channel membership
//! - `Publish` - Send messages to channels
//! - `Presence` - Track online users
//! - `Ack` / `Error` - Acknowledgments and errors
//!
//! ## Example
//!
//! ```rust
//! use pulse_protocol::{Frame, codec};
//!
//! // Create a publish frame using the helper method
//! let frame = Frame::publish("chat:lobby", b"Hello, world!".to_vec());
//!
//! // Encode and decode
//! let encoded = codec::encode(&frame).unwrap();
//! let decoded = codec::decode(&encoded).unwrap();
//! ```

pub mod codec;
pub mod frames;
pub mod version;

pub use codec::{decode, encode, ProtocolError};
pub use frames::{Frame, PresenceAction};
pub use version::{Version, PROTOCOL_VERSION};
