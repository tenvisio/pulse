//! # pulse-transport
//!
//! Transport abstraction layer for the Pulse realtime engine.
//!
//! This crate provides a unified interface for different transport protocols:
//!
//! - **WebSocket** - The standard, works everywhere
//! - **WebTransport** - HTTP/3 + QUIC for maximum performance
//!
//! ## Transport Abstraction
//!
//! All transports implement the `Transport` and `Connection` traits,
//! allowing the server to be protocol-agnostic.
//!
//! ```rust,ignore
//! use pulse_transport::{Transport, Connection};
//!
//! async fn handle_connection(mut conn: Box<dyn Connection>) {
//!     while let Ok(frame) = conn.recv().await {
//!         // Process frame
//!     }
//! }
//! ```

pub mod fallback;
pub mod traits;

#[cfg(feature = "websocket")]
pub mod websocket;

#[cfg(feature = "webtransport")]
pub mod webtransport;

pub use traits::{Connection, ConnectionId, Transport, TransportError};

#[cfg(feature = "websocket")]
pub use websocket::WebSocketTransport;
