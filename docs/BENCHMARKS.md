# Benchmarks

This document describes Pulse's benchmark methodology and results.

## Running Benchmarks

### Micro-benchmarks (Criterion)

```bash
# Run all benchmarks
cargo bench -p pulse-bench

# Run specific benchmark file
cargo bench -p pulse-bench --bench throughput
cargo bench -p pulse-bench --bench latency

# Run protocol codec benchmarks
cargo bench -p pulse-protocol --bench codec

# Quick test (verify benchmarks run without measuring)
cargo bench -p pulse-bench -- --test
```

### End-to-End Throughput Benchmark

Measures actual WebSocket message throughput with real network I/O:

```bash
# Terminal 1: Start the server
cargo run --release -p pulse-server

# Terminal 2: Run e2e benchmark (default: 16 clients)
cargo run --release -p pulse-bench --bin e2e_throughput

# With custom client count
cargo run --release -p pulse-bench --bin e2e_throughput -- 64
cargo run --release -p pulse-bench --bin e2e_throughput -- 128
```

The e2e benchmark:
- Connects multiple WebSocket clients to the server
- All clients subscribe to a shared "benchmark" channel
- Each client publishes messages while receiving from others
- Measures total message throughput over a 10-second window

## Results

Benchmarks run on Apple M2 MacBook Air. Results will vary by hardware.

### Codec Performance

| Operation | Size | Time | Throughput |
|-----------|------|------|------------|
| Encode | 64B | 217ns | 280 MiB/s |
| Encode | 1KB | 241ns | 3.9 GiB/s |
| Encode | 64KB | 4.9µs | 12.3 GiB/s |
| Decode | 64B | 150ns | 719 MiB/s |
| Decode | 1KB | 178ns | 5.6 GiB/s |
| Decode | 64KB | 1.7µs | 35.1 GiB/s |

### Router Performance

| Operation | Time |
|-----------|------|
| Subscribe | 163ns |
| Publish (1 subscriber) | 87ns |
| Publish (100 subscribers) | 87ns |
| Publish (1000 subscribers) | 87ns |
| Pub/sub roundtrip | 170ns |

### Fanout Performance

| Subscribers | Time | Throughput |
|-------------|------|------------|
| 10 | 88ns | 112M elem/s |
| 100 | 88ns | 1.1B elem/s |
| 1,000 | 88ns | 11.4B elem/s |
| 10,000 | 88ns | 113B elem/s |

> **Note**: Publish time is constant regardless of subscriber count thanks to efficient broadcast channels.

### Latency Benchmarks

| Operation | Time |
|-----------|------|
| Codec roundtrip (256B) | 419ns |
| Message creation (simple) | 101ns |
| Message creation (with metadata) | 142ns |
| Frame creation (subscribe) | 25ns |
| Frame creation (publish) | 47ns |
| Frame creation (ack) | 2ns |
| Subscription lookup | 49ns |

### Comparison

| System | Typical Latency | Notes |
|--------|-----------------|-------|
| **Pulse** | ~100-400ns | Local operations |
| NATS | 10-50µs | |
| Redis Pub/Sub | 10-100µs | |
| Socket.io | 1-10ms | |

## Profiling

### CPU Profiling

```bash
# With perf (Linux)
perf record -g ./target/release/pulse
perf report

# With flamegraph
cargo install flamegraph
cargo flamegraph -p pulse-server
```

### Memory Profiling

```bash
# With heaptrack (Linux)
heaptrack ./target/release/pulse
heaptrack_gui heaptrack.pulse.*.gz
```

## Optimization Notes

### Hot Paths

1. **Frame Encoding/Decoding**: Pre-sized buffers
2. **Channel Lookup**: DashMap with sharding
3. **Message Broadcast**: Zero-copy Bytes sharing via tokio broadcast

### Design Decisions

- **Constant-time fanout**: Using `tokio::sync::broadcast` channels means publish time doesn't scale with subscriber count
- **Lock-free routing**: DashMap provides concurrent access without global locks
- **Zero-copy payloads**: `bytes::Bytes` allows sharing payload data across subscribers

## Benchmark Code Location

```
crates/
├── pulse-bench/
│   ├── benches/
│   │   ├── throughput.rs      # Criterion throughput benchmarks
│   │   └── latency.rs         # Criterion latency benchmarks
│   └── src/bin/
│       └── e2e_throughput.rs  # End-to-end WebSocket benchmark
└── pulse-protocol/
    └── benches/
        └── codec.rs           # Codec benchmarks
```
