//! End-to-end throughput benchmark for Pulse.
//!
//! This benchmark measures actual WebSocket message throughput with real network I/O.

use bytes::BytesMut;
use futures_util::{SinkExt, StreamExt};
use pulse_protocol::{codec, Frame};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Barrier;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const SERVER_URL: &str = "ws://127.0.0.1:8080/ws";
const WARMUP_SECS: u64 = 2;
const BENCH_SECS: u64 = 10;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let num_clients = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(16);

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║         Pulse End-to-End Throughput Benchmark                ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Make sure the server is running: cargo run --release        ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    run_pubsub_benchmark(num_clients).await;
}

async fn run_pubsub_benchmark(num_clients: usize) {
    println!("📊 Pub/Sub Benchmark: {} clients", num_clients);
    println!("   Warmup: {}s, Measurement: {}s", WARMUP_SECS, BENCH_SECS);
    println!();

    let message_count = Arc::new(AtomicU64::new(0));
    let barrier = Arc::new(Barrier::new(num_clients + 1));

    let mut handles = Vec::new();

    // Spawn client tasks
    for client_id in 0..num_clients {
        let msg_count = Arc::clone(&message_count);
        let barrier = Arc::clone(&barrier);

        let handle = tokio::spawn(async move {
            if let Err(e) = run_client(client_id, msg_count, barrier).await {
                eprintln!("Client {} error: {}", client_id, e);
            }
        });
        handles.push(handle);
    }

    // Wait for all clients to connect
    barrier.wait().await;
    println!("✓ All {} clients connected", num_clients);

    // Warmup phase
    println!("⏳ Warming up for {}s...", WARMUP_SECS);
    tokio::time::sleep(Duration::from_secs(WARMUP_SECS)).await;

    // Reset counter and start measurement
    message_count.store(0, Ordering::SeqCst);
    let start = Instant::now();

    println!("📈 Measuring for {}s...", BENCH_SECS);
    tokio::time::sleep(Duration::from_secs(BENCH_SECS)).await;

    let elapsed = start.elapsed();
    let total_messages = message_count.load(Ordering::SeqCst);

    // Calculate throughput
    let msgs_per_sec = total_messages as f64 / elapsed.as_secs_f64();
    let msgs_per_sec_per_client = msgs_per_sec / num_clients as f64;

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                         RESULTS                              ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!(
        "║  Clients:              {:>10}                           ║",
        num_clients
    );
    println!(
        "║  Duration:             {:>10.2}s                          ║",
        elapsed.as_secs_f64()
    );
    println!(
        "║  Total Messages:       {:>10}                           ║",
        total_messages
    );
    println!(
        "║  Throughput:           {:>10.0} msg/s                    ║",
        msgs_per_sec
    );
    println!(
        "║  Per-Client:           {:>10.0} msg/s                    ║",
        msgs_per_sec_per_client
    );
    println!("╚══════════════════════════════════════════════════════════════╝");

    // Signal clients to stop
    for handle in handles {
        handle.abort();
    }
}

async fn run_client(
    client_id: usize,
    message_count: Arc<AtomicU64>,
    barrier: Arc<Barrier>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Connect to server
    let (ws, _) = connect_async(SERVER_URL).await?;
    let (mut sender, mut receiver) = ws.split();

    // Wait for Connected frame from server
    if let Some(Ok(_connected)) = receiver.next().await {
        // Got Connected frame
    }

    // Subscribe to broadcast channel using proper Pulse protocol
    let subscribe_frame = Frame::subscribe(client_id as u64, "benchmark");
    let subscribe_bytes = codec::encode(&subscribe_frame)?;
    sender
        .send(Message::Binary(subscribe_bytes.to_vec()))
        .await?;

    // Wait for Subscribe Ack
    if let Some(Ok(_ack)) = receiver.next().await {
        // Got Ack, subscription is ready
    }

    // Wait for all clients to be ready
    barrier.wait().await;

    // Pre-encode the publish frame for efficiency
    let payload = vec![0u8; 64];
    let publish_frame = Frame::publish("benchmark", payload);
    let publish_bytes = codec::encode(&publish_frame)?;
    let publish_msg = Message::Binary(publish_bytes.to_vec());

    // Spawn separate receiver task for full-duplex operation
    let recv_count = message_count.clone();
    let recv_task = tokio::spawn(async move {
        let mut recv_buf = BytesMut::with_capacity(65536);

        while let Some(result) = receiver.next().await {
            if let Ok(Message::Binary(data)) = result {
                recv_buf.extend_from_slice(&data);
                // Decode all complete frames
                while let Ok(Some(_frame)) = codec::decode_from(&mut recv_buf) {
                    recv_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    });

    // Send loop - no waiting, just blast messages
    loop {
        if sender.send(publish_msg.clone()).await.is_err() {
            break;
        }
        // Small yield to not starve the receiver task
        tokio::task::yield_now().await;
    }

    recv_task.abort();
    Ok(())
}
