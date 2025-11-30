//! WebSocket transport implementation.
//!
//! This module provides a WebSocket-based transport using tokio-tungstenite.

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use futures_util::{SinkExt, StreamExt};
use pulse_protocol::{codec, Frame};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Error as WsError, Message},
    WebSocketStream,
};
use tracing::{debug, error, info, warn};

use crate::traits::{Connection, ConnectionId, Transport, TransportError};

/// WebSocket transport configuration.
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// Address to bind to.
    pub bind_addr: SocketAddr,
    /// Maximum message size in bytes.
    pub max_message_size: usize,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            max_message_size: 64 * 1024, // 64 KB
        }
    }
}

/// WebSocket transport.
pub struct WebSocketTransport {
    listener: TcpListener,
    config: WebSocketConfig,
}

impl WebSocketTransport {
    /// Create a new WebSocket transport.
    ///
    /// # Errors
    ///
    /// Returns an error if binding to the address fails.
    pub async fn new(config: WebSocketConfig) -> Result<Self, TransportError> {
        let listener = TcpListener::bind(config.bind_addr)
            .await
            .map_err(TransportError::Io)?;

        info!("WebSocket transport listening on {}", config.bind_addr);

        Ok(Self { listener, config })
    }

    /// Create a new WebSocket transport with default config.
    ///
    /// # Errors
    ///
    /// Returns an error if binding fails.
    pub async fn bind(addr: SocketAddr) -> Result<Self, TransportError> {
        Self::new(WebSocketConfig {
            bind_addr: addr,
            ..Default::default()
        })
        .await
    }

    /// Get the local address this transport is bound to.
    #[must_use]
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.listener.local_addr().ok()
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    async fn accept(&self) -> Result<Box<dyn Connection>, TransportError> {
        let (stream, addr) = self.listener.accept().await.map_err(TransportError::Io)?;

        debug!("Accepted TCP connection from {}", addr);

        let ws_stream = accept_async(stream).await.map_err(|e| {
            error!("WebSocket handshake failed: {}", e);
            TransportError::Other(format!("WebSocket handshake failed: {}", e))
        })?;

        debug!("WebSocket handshake completed with {}", addr);

        let conn = WebSocketConnection::new(ws_stream, addr, self.config.max_message_size);
        Ok(Box::new(conn))
    }

    fn name(&self) -> &'static str {
        "websocket"
    }
}

/// A WebSocket connection.
pub struct WebSocketConnection {
    id: ConnectionId,
    stream: Arc<Mutex<WebSocketStream<TcpStream>>>,
    remote_addr: SocketAddr,
    is_open: AtomicBool,
    read_buffer: BytesMut,
    max_message_size: usize,
}

impl WebSocketConnection {
    /// Create a new WebSocket connection.
    fn new(
        stream: WebSocketStream<TcpStream>,
        remote_addr: SocketAddr,
        max_message_size: usize,
    ) -> Self {
        Self {
            id: ConnectionId::generate(),
            stream: Arc::new(Mutex::new(stream)),
            remote_addr,
            is_open: AtomicBool::new(true),
            read_buffer: BytesMut::with_capacity(4096),
            max_message_size,
        }
    }
}

#[async_trait]
impl Connection for WebSocketConnection {
    fn id(&self) -> &ConnectionId {
        &self.id
    }

    async fn recv(&mut self) -> Result<Option<Frame>, TransportError> {
        // First, try to decode from the existing buffer
        if let Some(frame) = codec::decode_from(&mut self.read_buffer)? {
            return Ok(Some(frame));
        }

        // Need more data - read from the WebSocket
        let mut stream = self.stream.lock().await;

        loop {
            match stream.next().await {
                Some(Ok(Message::Binary(data))) => {
                    if data.len() > self.max_message_size {
                        warn!(
                            "Message too large: {} bytes (max: {})",
                            data.len(),
                            self.max_message_size
                        );
                        return Err(TransportError::Protocol(
                            pulse_protocol::ProtocolError::FrameTooLarge(data.len()),
                        ));
                    }

                    self.read_buffer.extend_from_slice(&data);

                    // Try to decode a frame
                    if let Some(frame) = codec::decode_from(&mut self.read_buffer)? {
                        return Ok(Some(frame));
                    }
                    // Need more data, continue reading
                }
                Some(Ok(Message::Text(text))) => {
                    // For compatibility, treat text as binary
                    self.read_buffer.extend_from_slice(text.as_bytes());

                    if let Some(frame) = codec::decode_from(&mut self.read_buffer)? {
                        return Ok(Some(frame));
                    }
                }
                Some(Ok(Message::Ping(data))) => {
                    // Respond to ping with pong
                    if let Err(e) = stream.send(Message::Pong(data)).await {
                        warn!("Failed to send pong: {}", e);
                    }
                }
                Some(Ok(Message::Pong(_))) => {
                    // Ignore pong messages
                }
                Some(Ok(Message::Close(_))) => {
                    debug!("Received close frame");
                    self.is_open.store(false, Ordering::SeqCst);
                    return Ok(None);
                }
                Some(Ok(Message::Frame(_))) => {
                    // Raw frame, ignore
                }
                Some(Err(WsError::ConnectionClosed)) => {
                    debug!("Connection closed");
                    self.is_open.store(false, Ordering::SeqCst);
                    return Ok(None);
                }
                Some(Err(e)) => {
                    error!("WebSocket error: {}", e);
                    self.is_open.store(false, Ordering::SeqCst);
                    return Err(TransportError::ReceiveFailed(e.to_string()));
                }
                None => {
                    debug!("WebSocket stream ended");
                    self.is_open.store(false, Ordering::SeqCst);
                    return Ok(None);
                }
            }
        }
    }

    async fn send(&mut self, frame: Frame) -> Result<(), TransportError> {
        let data = codec::encode(&frame)?;
        self.send_raw(data).await
    }

    async fn send_raw(&mut self, data: Bytes) -> Result<(), TransportError> {
        if !self.is_open.load(Ordering::SeqCst) {
            return Err(TransportError::ConnectionClosed);
        }

        let mut stream = self.stream.lock().await;
        stream
            .send(Message::Binary(data.to_vec()))
            .await
            .map_err(|e| TransportError::SendFailed(e.to_string()))
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        if !self.is_open.swap(false, Ordering::SeqCst) {
            return Ok(()); // Already closed
        }

        let mut stream = self.stream.lock().await;
        stream
            .close(None)
            .await
            .map_err(|e| TransportError::Other(format!("Failed to close: {}", e)))
    }

    fn remote_addr(&self) -> Option<String> {
        Some(self.remote_addr.to_string())
    }

    fn is_open(&self) -> bool {
        self.is_open.load(Ordering::SeqCst)
    }
}

/// Upgrade an HTTP request to a WebSocket connection.
///
/// This is useful when integrating with axum or other HTTP frameworks.
pub async fn upgrade_to_websocket(
    stream: TcpStream,
    max_message_size: usize,
) -> Result<WebSocketConnection, TransportError> {
    let addr = stream.peer_addr().map_err(TransportError::Io)?;

    let ws_stream = accept_async(stream)
        .await
        .map_err(|e| TransportError::Other(format!("WebSocket handshake failed: {}", e)))?;

    Ok(WebSocketConnection::new(ws_stream, addr, max_message_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketConfig::default();
        assert_eq!(config.bind_addr.port(), 8080);
        assert_eq!(config.max_message_size, 64 * 1024);
    }
}
