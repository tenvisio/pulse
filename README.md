# Pulse

[![CI](https://github.com/tenvisio/pulse/workflows/CI/badge.svg)](https://github.com/tenvisio/pulse/actions)
[![Crates.io](https://img.shields.io/crates/v/pulse-server.svg)](https://crates.io/crates/pulse-server)
[![Documentation](https://docs.rs/pulse-core/badge.svg)](https://docs.rs/pulse-core)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

**The world's fastest realtime communication engine for web and edge applications.**

Pulse is a high-performance, transport-agnostic pub/sub messaging system built in Rust. It's designed for applications that need sub-millisecond latency and massive scale.

## Features

- **Blazing Fast**: Lock-free data structures, zero-copy message passing
- **Transport Agnostic**: WebSocket today, WebTransport tomorrow
- **Protocol Efficient**: Binary MessagePack encoding
- **Developer Friendly**: Simple pub/sub API
- **Observable**: Built-in Prometheus metrics
- **Production Ready**: Comprehensive error handling and logging

## Quick Start

### Installation

```bash
# From crates.io
cargo install pulse-server

# From source
git clone https://github.com/tenvisio/pulse
cd pulse
cargo build --release
```

### Run the Server

```bash
# With defaults (localhost:8080)
pulse

# With custom port
PULSE_PORT=9000 pulse

# With config file
pulse --config pulse.toml
```

### Connect a Client

Messages use MessagePack encoding with a 4-byte length prefix. Here's the message structure:

```javascript
// Subscribe to a channel
{ type: 'subscribe', id: 1, channel: 'chat:lobby' }

// Publish a message
{ type: 'publish', channel: 'chat:lobby', payload: /* msgpack bytes */ }

// Unsubscribe
{ type: 'unsubscribe', id: 2, channel: 'chat:lobby' }
```

> **Note**: The wire format requires a 4-byte big-endian length prefix before each MessagePack frame.
> See [`examples/web-client/`](examples/web-client/) for a complete working browser implementation,
> or wait for the upcoming TypeScript SDK.

## Architecture

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

## Crates

| Crate | Description |
|-------|-------------|
| [`pulse-protocol`](crates/pulse-protocol) | Wire protocol definitions and codec |
| [`pulse-core`](crates/pulse-core) | Router, channels, and presence |
| [`pulse-transport`](crates/pulse-transport) | Transport abstractions (WebSocket, WebTransport) |
| [`pulse-server`](crates/pulse-server) | The server binary |
| [`pulse-bench`](crates/pulse-bench) | Performance benchmarks |

## Configuration

Create a `pulse.toml` file:

```toml
[server]
host = "0.0.0.0"
port = 8080

[transport]
websocket = true
webtransport = false  # Experimental

[limits]
max_connections = 100000
max_channels = 10000
max_subscriptions_per_connection = 100
max_message_size = 65536  # 64 KB

[heartbeat]
interval_ms = 30000
timeout_ms = 60000

[metrics]
enabled = true
port = 9090
```

Or use environment variables:

```bash
export PULSE_HOST=0.0.0.0
export PULSE_PORT=8080
export PULSE_LIMITS_MAX_CONNECTIONS=100000
```

## Protocol

Pulse uses a binary protocol based on MessagePack. See the full [Protocol Specification](docs/PROTOCOL.md).

### Frame Types

| Type | Direction | Description |
|------|-----------|-------------|
| Subscribe | Client → Server | Subscribe to a channel |
| Unsubscribe | Client → Server | Unsubscribe from a channel |
| Publish | Bidirectional | Send message to channel |
| Presence | Bidirectional | Presence updates |
| Ack | Server → Client | Acknowledgment |
| Error | Server → Client | Error response |
| Ping/Pong | Bidirectional | Keepalive |

## Benchmarks

Run benchmarks:

```bash
# Run micro-benchmarks (Criterion)
cargo bench -p pulse-bench

# Run specific benchmark file
cargo bench -p pulse-bench --bench throughput
cargo bench -p pulse-bench --bench latency

# Run end-to-end throughput test (requires server running)
# Terminal 1: Start the server
cargo run --release -p pulse-server

# Terminal 2: Run e2e benchmark with 16 clients (default)
cargo run --release -p pulse-bench --bin e2e_throughput

# Or with custom client count
cargo run --release -p pulse-bench --bin e2e_throughput -- 64
```

### Results

| Metric | Time | Throughput |
|--------|------|------------|
| Encode 64B message | 217ns | 280 MiB/s |
| Encode 1KB message | 241ns | 3.9 GiB/s |
| Encode 64KB message | 4.9µs | 12.3 GiB/s |
| Decode 64B message | 150ns | 719 MiB/s |
| Decode 1KB message | 178ns | 5.6 GiB/s |
| Decode 64KB message | 1.7µs | 35.1 GiB/s |
| Publish (any subscriber count) | 87ns | — |
| Subscribe operation | 163ns | — |
| Pub/sub roundtrip | 170ns | — |

#### Fanout Performance

| Subscribers | Time | Throughput |
|-------------|------|------------|
| 10 | 88ns | 112M elem/s |
| 100 | 88ns | 1.1B elem/s |
| 1,000 | 88ns | 11.4B elem/s |
| 10,000 | 88ns | 113B elem/s |

> **Note**: Publish time is constant regardless of subscriber count thanks to efficient broadcast channels.

*Benchmarks run on Apple M2 MacBook Air, results will vary.*

## Comparison

| Feature | Pulse | Socket.io | Phoenix Channels |
|---------|-------|-----------|------------------|
| Language | Rust | Node.js | Elixir |
| Protocol | Binary (MessagePack) | JSON | JSON/Binary |
| Transport | WS, WebTransport | WS, Polling | WS |
| Latency | Sub-ms | ~10-50ms | ~1-5ms |
| Memory per connection | ~1KB | ~10KB | ~5KB |

## Roadmap

- [x] Core pub/sub router
- [x] WebSocket transport
- [x] Presence tracking
- [x] Prometheus metrics
- [ ] Docker images
- [ ] WebTransport support
- [ ] Redis adapter for clustering
- [ ] Message persistence/replay
- [ ] TypeScript client SDK
- [ ] Authentication hooks
- [ ] Rate limiting

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md).

```bash
# Setup
git clone https://github.com/tenvisio/pulse
cd pulse
cargo build

# Run tests
cargo test

# Run clippy
cargo clippy -- -D warnings

# Format code
cargo fmt
```

## License

Pulse is dual-licensed under the [MIT License](LICENSE-MIT) and [Apache License 2.0](LICENSE-APACHE).

## Acknowledgments

Built with:
- [Tokio](https://tokio.rs/) - Async runtime
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [DashMap](https://github.com/xacrimon/dashmap) - Concurrent hashmap
- [MessagePack](https://msgpack.org/) - Efficient serialization

---

<p align="center">
<sub>Built by <a href="https://github.com/tenvisio"><b>tenvisio</b></a></sub>
</p>

---


