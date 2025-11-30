# Pulse Architecture

This document describes the high-level architecture of Pulse.

## Overview

Pulse is designed as a modular, layered system:

```
┌────────────────────────────────────────────────────────────┐
│                      pulse-server                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Config    │  │   Metrics   │  │      Handlers       │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────┐
│                       pulse-core                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Router    │  │   Channel   │  │      Presence       │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────┐
│                    pulse-transport                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │  WebSocket  │  │ WebTransport│  │     Fallback        │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────┐
│                     pulse-protocol                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Frames    │  │    Codec    │  │      Version        │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

## Crate Responsibilities

### pulse-protocol

The lowest layer, defining the wire protocol:

- **Frames**: All message types (Subscribe, Publish, Presence, etc.)
- **Codec**: MessagePack encoding/decoding
- **Version**: Protocol versioning and negotiation

This crate has minimal dependencies and can be used by client implementations.

### pulse-transport

Abstracts different transport protocols:

- **Traits**: `Transport` and `Connection` traits
- **WebSocket**: tokio-tungstenite implementation
- **WebTransport**: wtransport implementation (experimental)
- **Fallback**: Auto-negotiation between transports

### pulse-core

The application logic layer:

- **Router**: High-performance pub/sub message routing
- **Channel**: Room/topic abstraction
- **Presence**: Track and broadcast user presence
- **Message**: Internal message types

### pulse-server

The executable binary:

- **Config**: TOML and environment configuration
- **Handlers**: Connection lifecycle management
- **Metrics**: Prometheus metrics export

## Data Flow

### Message Publishing

```
1. Client sends Publish frame
           │
           ▼
2. Transport decodes frame
           │
           ▼
3. Router looks up channel subscribers
           │
           ▼
4. For each subscriber:
   4a. Encode frame
   4b. Send via their transport
```

### Subscription

```
1. Client sends Subscribe frame
           │
           ▼
2. Router validates channel name
           │
           ▼
3. Router adds connection to channel's subscriber set
           │
           ▼
4. Send Ack frame to client
           │
           ▼
5. (If presence) Broadcast join to other subscribers
```

## Concurrency Model

Pulse uses Tokio for async I/O with a specific concurrency strategy:

### Connection Handling

Each connection runs in its own Tokio task:

```rust
tokio::spawn(async move {
    loop {
        let frame = connection.recv().await?;
        handle_frame(frame, &router).await;
    }
});
```

### Channel State

Channels use `DashMap` for lock-free concurrent access:

```rust
pub struct Router {
    channels: DashMap<ChannelId, Channel>,
}
```

### Broadcasting

Each channel uses Tokio broadcast channels:

```rust
pub struct Channel {
    sender: broadcast::Sender<Message>,
    // Receivers are created per-subscriber
}
```

## Memory Management

### Zero-Copy Design

Pulse minimizes allocations using `bytes::Bytes`:

1. **Incoming**: Read into buffer, parse without copying
2. **Routing**: Share `Bytes` reference across subscribers
3. **Outgoing**: Encode once, send to all

### Buffer Pooling

For high-throughput scenarios:

- Reuse encoding buffers
- Pool connection read buffers
- Avoid allocations in hot paths

## Configuration

Pulse uses a layered configuration approach:

```
Environment Variables
        │
        ▼ (overrides)
   Config File
        │
        ▼ (overrides)
      Defaults
```

### Example Configuration

```toml
[server]
host = "0.0.0.0"
port = 8080

[transport]
websocket = true
webtransport = false

[limits]
max_connections = 10000
max_channels = 1000
max_message_size = 65536

[heartbeat]
interval = 30000
timeout = 60000
```

## Metrics

Pulse exports Prometheus metrics:

| Metric | Type | Description |
|--------|------|-------------|
| `pulse_connections_total` | Counter | Total connections |
| `pulse_connections_active` | Gauge | Current connections |
| `pulse_messages_total` | Counter | Messages processed |
| `pulse_messages_bytes` | Counter | Bytes transferred |
| `pulse_channels_active` | Gauge | Active channels |
| `pulse_latency_seconds` | Histogram | Message latency |

## Scaling

### Vertical Scaling

Single server optimizations:

- Increase file descriptor limits
- Tune TCP buffers
- Use multiple Tokio worker threads

### Horizontal Scaling (Future)

For distributed deployments:

1. **Sticky Sessions**: Route clients to same server
2. **Message Bus**: Redis/NATS for cross-server messaging
3. **Presence Sync**: CRDT-based presence synchronization

## Security

### Connection Level

- TLS termination (recommended: reverse proxy)
- Connection rate limiting
- Maximum connections per IP

### Application Level

- Token authentication in Connect frame
- Per-channel authorization
- Message size limits

## Future Considerations

1. **Clustering**: Redis adapter for horizontal scaling
2. **Persistence**: Message history and replay
3. **Webhooks**: HTTP callbacks for events
4. **Admin API**: Runtime configuration and monitoring





