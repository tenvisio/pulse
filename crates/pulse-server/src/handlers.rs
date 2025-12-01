//! Connection handlers for Pulse server.
//!
//! This module handles the connection lifecycle and message processing.

use crate::config::Config;
use crate::metrics::{self, ConnectionMetricsGuard};
use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use bytes::BytesMut;
use futures_util::{SinkExt, StreamExt};
use tenvis_pulse_core::{Router as PulseRouter, RouterConfig};
use pulse_protocol::{codec, Frame};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// Shared server state.
pub struct AppState {
    /// The message router.
    pub router: PulseRouter,
    /// Server configuration.
    pub config: Config,
}

impl AppState {
    /// Create new app state.
    #[must_use]
    pub fn new(config: Config) -> Self {
        let router_config = RouterConfig {
            max_channels: config.limits.max_channels,
            max_subscriptions_per_connection: config.limits.max_subscriptions_per_connection,
            channel_capacity: 131072,
            auto_create_channels: true,
            auto_delete_empty_channels: true,
        };

        Self {
            router: PulseRouter::with_config(router_config),
            config,
        }
    }
}

/// Run the HTTP/WebSocket server.
///
/// # Errors
///
/// Returns an error if the server fails to start.
pub async fn run_server(config: Config) -> Result<()> {
    let state = Arc::new(AppState::new(config.clone()));

    // Start metrics server if enabled
    if config.metrics.enabled {
        if let Err(e) = metrics::start_metrics_server(config.metrics.port) {
            error!("Failed to start metrics server: {}", e);
        }
    }

    // Build router
    let app = Router::new()
        .route(&config.transport.websocket_path, get(ws_handler))
        .route("/health", get(health_handler))
        .with_state(state);

    // Bind and serve
    let addr = config.bind_addr();
    let listener = TcpListener::bind(addr).await?;

    info!("Pulse server listening on {}", addr);
    info!(
        "WebSocket endpoint: ws://{}{}",
        addr, config.transport.websocket_path
    );

    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check handler.
async fn health_handler() -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// WebSocket upgrade handler.
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

/// Handle a WebSocket connection.
async fn handle_websocket(socket: WebSocket, state: Arc<AppState>) {
    // Record connection metrics
    let _metrics_guard = ConnectionMetricsGuard::new();

    // Generate connection ID
    let connection_id = format!(
        "conn_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    debug!(connection = %connection_id, "WebSocket connected");

    // Split the WebSocket
    let (mut sender, mut receiver) = socket.split();

    // Send Connected frame
    let connected_frame =
        Frame::connected(&connection_id, 1, state.config.heartbeat.interval_ms as u32);
    if let Ok(data) = codec::encode(&connected_frame) {
        if sender.send(Message::Binary(data.to_vec())).await.is_err() {
            error!(connection = %connection_id, "Failed to send Connected frame");
            return;
        }
    }

    // Read buffer for partial frames
    let mut read_buffer = BytesMut::with_capacity(4096);

    // Track subscription task handles for cleanup
    let mut subscription_tasks: HashMap<String, tokio::task::JoinHandle<()>> = HashMap::new();

    // Create a merged stream for all subscription receivers
    let (sub_tx, mut sub_rx) =
        tokio::sync::mpsc::unbounded_channel::<(String, Arc<tenvis_pulse_core::Message>)>();

    // Message processing loop
    loop {
        tokio::select! {
            biased;

            // Receive messages from subscribed channels (via mpsc)
            Some((channel, msg)) = sub_rx.recv() => {
                // Forward the message to the WebSocket client
                let frame = Frame::Publish {
                    id: None,
                    channel,
                    event: msg.event.clone(),
                    payload: msg.payload.to_vec(),
                };
                if let Ok(data) = codec::encode(&frame) {
                    metrics::record_message(data.len(), "outbound");
                    if sender.send(Message::Binary(data.to_vec())).await.is_err() {
                        break;
                    }
                }
            }

            // Receive from WebSocket
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        let start = Instant::now();
                        read_buffer.extend_from_slice(&data);

                        // Try to decode frames
                        while let Ok(Some(frame)) = codec::decode_from(&mut read_buffer) {
                            metrics::record_message(data.len(), "inbound");

                            if let Err(e) = handle_frame(
                                &frame,
                                &connection_id,
                                &state,
                                &mut sender,
                                &mut subscription_tasks,
                                &sub_tx,
                            ).await {
                                error!(connection = %connection_id, error = %e, "Frame handling error");
                                break;
                            }
                        }

                        metrics::record_latency(start.elapsed().as_secs_f64());
                    }
                    Some(Ok(Message::Text(text))) => {
                        // Treat text as binary
                        read_buffer.extend_from_slice(text.as_bytes());
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // Ignore pongs
                    }
                    Some(Ok(Message::Close(_))) => {
                        debug!(connection = %connection_id, "Received close frame");
                        break;
                    }
                    Some(Err(e)) => {
                        warn!(connection = %connection_id, error = %e, "WebSocket error");
                        metrics::record_error("websocket");
                        break;
                    }
                    None => {
                        debug!(connection = %connection_id, "WebSocket stream ended");
                        break;
                    }
                }
            }
        }
    }

    // Cleanup: abort all subscription tasks
    for (_, handle) in subscription_tasks {
        handle.abort();
    }

    // Cleanup: unsubscribe from all channels
    state.router.unsubscribe_all(&connection_id);
    metrics::set_active_channels(state.router.stats().channel_count);

    debug!(connection = %connection_id, "WebSocket disconnected");
}

/// Handle a decoded frame.
async fn handle_frame(
    frame: &Frame,
    connection_id: &str,
    state: &Arc<AppState>,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    subscription_tasks: &mut HashMap<String, tokio::task::JoinHandle<()>>,
    sub_tx: &tokio::sync::mpsc::UnboundedSender<(String, Arc<tenvis_pulse_core::Message>)>,
) -> Result<()> {
    match frame {
        Frame::Subscribe { id, channel } => {
            debug!(connection = %connection_id, channel = %channel, "Subscribe request");

            let response = match state.router.subscribe(connection_id, channel) {
                Ok(mut rx) => {
                    // Spawn a task to forward messages from broadcast to mpsc
                    let channel_name = channel.clone();
                    let tx = sub_tx.clone();
                    let handle = tokio::spawn(async move {
                        loop {
                            match rx.recv().await {
                                Ok(msg) => {
                                    if tx.send((channel_name.clone(), msg)).is_err() {
                                        break; // Receiver dropped
                                    }
                                }
                                Err(broadcast::error::RecvError::Closed) => break,
                                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                            }
                        }
                    });
                    subscription_tasks.insert(channel.clone(), handle);
                    metrics::record_subscription();
                    metrics::set_active_channels(state.router.stats().channel_count);
                    Frame::ack(*id)
                }
                Err(e) => {
                    warn!(connection = %connection_id, error = %e, "Subscribe failed");
                    Frame::error(*id, 1002, e.to_string())
                }
            };

            send_frame(sender, &response).await?;
        }

        Frame::Unsubscribe { id, channel } => {
            debug!(connection = %connection_id, channel = %channel, "Unsubscribe request");

            // Abort the subscription task
            if let Some(handle) = subscription_tasks.remove(channel) {
                handle.abort();
            }

            let response = match state.router.unsubscribe(connection_id, channel) {
                Ok(()) => {
                    metrics::set_active_channels(state.router.stats().channel_count);
                    Frame::ack(*id)
                }
                Err(e) => Frame::error(*id, 1008, e.to_string()),
            };

            send_frame(sender, &response).await?;
        }

        Frame::Publish {
            id,
            channel,
            event,
            payload,
        } => {
            debug!(connection = %connection_id, channel = %channel, "Publish");

            let mut message = tenvis_pulse_core::Message::new(channel.clone(), payload.clone())
                .with_source(connection_id);

            if let Some(evt) = event {
                message = message.with_event(evt.clone());
            }

            let count = state.router.publish(message);
            metrics::record_message(payload.len(), "broadcast");

            // Send ack if requested
            if let Some(req_id) = id {
                send_frame(sender, &Frame::ack(*req_id)).await?;
            }

            debug!(connection = %connection_id, channel = %channel, recipients = count, "Published");
        }

        Frame::Ping { timestamp } => {
            send_frame(sender, &Frame::pong(*timestamp)).await?;
        }

        Frame::Pong { .. } => {
            // Update last seen for presence
        }

        Frame::Connect { version, token } => {
            debug!(
                connection = %connection_id,
                version = version,
                has_token = token.is_some(),
                "Connect frame (already connected)"
            );
            // Connection already established, ignore
        }

        _ => {
            warn!(connection = %connection_id, frame_type = ?frame.frame_type(), "Unexpected frame type");
        }
    }

    Ok(())
}

/// Send a frame to the WebSocket.
async fn send_frame(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    frame: &Frame,
) -> Result<()> {
    let data = codec::encode(frame)?;
    metrics::record_message(data.len(), "outbound");
    sender.send(Message::Binary(data.to_vec())).await?;
    Ok(())
}
