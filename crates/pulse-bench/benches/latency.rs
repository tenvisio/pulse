//! Latency benchmarks for Pulse.
//!
//! These benchmarks focus on measuring end-to-end latency.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tenvis_pulse_core::{Message, Router};
use pulse_protocol::{codec, Frame};
use std::time::Instant;

/// Benchmark round-trip encode/decode latency.
fn bench_codec_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_roundtrip");

    let frame = Frame::publish("test:channel", vec![0u8; 256]);

    group.bench_function("256B", |b| {
        b.iter(|| {
            let encoded = codec::encode(black_box(&frame)).unwrap();
            codec::decode(black_box(&encoded)).unwrap()
        });
    });

    group.finish();
}

/// Benchmark subscribe + publish + receive latency.
fn bench_pubsub_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("pubsub_latency");

    group.bench_function("single_subscriber", |b| {
        b.iter_custom(|iters| {
            let router = Router::new();
            let mut rx = router.subscribe("conn-1", "test").unwrap();

            let start = Instant::now();
            for _ in 0..iters {
                let message = Message::new("test", vec![0u8; 64]);
                router.publish(message);
                let _ = rx.try_recv();
            }
            start.elapsed()
        });
    });

    group.bench_function("ten_subscribers", |b| {
        b.iter_custom(|iters| {
            let router = Router::new();
            let mut rxs: Vec<_> = (0..10)
                .map(|i| router.subscribe(&format!("conn-{}", i), "test").unwrap())
                .collect();

            let start = Instant::now();
            for _ in 0..iters {
                let message = Message::new("test", vec![0u8; 64]);
                router.publish(message);
                for rx in &mut rxs {
                    let _ = rx.try_recv();
                }
            }
            start.elapsed()
        });
    });

    group.finish();
}

/// Benchmark message creation latency.
fn bench_message_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_creation");

    group.bench_function("simple", |b| {
        b.iter(|| Message::new(black_box("test:channel"), black_box(vec![0u8; 64])))
    });

    group.bench_function("with_metadata", |b| {
        b.iter(|| {
            Message::new(black_box("test:channel"), black_box(vec![0u8; 64]))
                .with_source(black_box("conn-123"))
                .with_event(black_box("user:message"))
        })
    });

    group.finish();
}

/// Benchmark frame type creation.
fn bench_frame_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_creation");

    group.bench_function("subscribe", |b| {
        b.iter(|| Frame::subscribe(black_box(1), black_box("test:channel")))
    });

    group.bench_function("publish", |b| {
        b.iter(|| Frame::publish(black_box("test:channel"), black_box(vec![0u8; 64])))
    });

    group.bench_function("ack", |b| b.iter(|| Frame::ack(black_box(1))));

    group.bench_function("error", |b| {
        b.iter(|| Frame::error(black_box(1), black_box(1001), black_box("Error message")))
    });

    group.finish();
}

/// Benchmark concurrent subscription lookup.
fn bench_subscription_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("subscription_lookup");

    // Setup: 1000 channels with 10 subscribers each
    let router = Router::new();
    for i in 0..1000 {
        let channel = format!("channel:{}", i);
        for j in 0..10 {
            let conn = format!("conn:{}:{}", i, j);
            let _ = router.subscribe(&conn, &channel);
        }
    }

    group.bench_function("channel_exists", |b| {
        let mut i = 0;
        b.iter(|| {
            let channel = format!("channel:{}", i % 1000);
            i += 1;
            router.channel_exists(black_box(&channel))
        });
    });

    group.bench_function("subscriber_count", |b| {
        let mut i = 0;
        b.iter(|| {
            let channel = format!("channel:{}", i % 1000);
            i += 1;
            router.subscriber_count(black_box(&channel))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_codec_roundtrip,
    bench_pubsub_latency,
    bench_message_creation,
    bench_frame_creation,
    bench_subscription_lookup,
);
criterion_main!(benches);
