//! # pulse-core
//!
//! Core types, traits, and message routing for the Pulse realtime engine.
//!
//! This crate provides the fundamental building blocks:
//!
//! - **Channel** - Room/topic abstraction for grouping connections
//! - **Router** - High-performance pub/sub message routing
//! - **Presence** - Track and broadcast user presence
//! - **Message** - Internal message types
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │  Connection │────▶│   Router    │────▶│  Channel    │
//! └─────────────┘     └─────────────┘     └─────────────┘
//!                            │
//!                            ▼
//!                     ┌─────────────┐
//!                     │  Presence   │
//!                     └─────────────┘
//! ```

pub mod channel;
pub mod message;
pub mod presence;
pub mod router;

pub use channel::{Channel, ChannelId};
pub use message::Message;
pub use presence::{Presence, PresenceState};
pub use router::{Router, RouterConfig, RouterError};
