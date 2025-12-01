//! Throughput benchmarks for Pulse.
//!
//! These benchmarks measure the raw message throughput of various components.

use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pulse_protocol::{codec, Frame};
use tenvis_pulse_core::{Message, Router};

/// Benchmark frame encoding.
fn bench_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode");

    // Small message
    let small_payload = Bytes::from(vec![0u8; 64]);
    let small_frame = Frame::publish("test:channel", small_payload.clone());
    group.throughput(Throughput::Bytes(64));
    group.bench_function("64B", |b| b.iter(|| codec::encode(black_box(&small_frame))));

    // Medium message
    let medium_payload = Bytes::from(vec![0u8; 1024]);
    let medium_frame = Frame::publish("test:channel", medium_payload.clone());
    group.throughput(Throughput::Bytes(1024));
    group.bench_function("1KB", |b| {
        b.iter(|| codec::encode(black_box(&medium_frame)))
    });

    // Large message
    let large_payload = Bytes::from(vec![0u8; 65536]);
    let large_frame = Frame::publish("test:channel", large_payload.clone());
    group.throughput(Throughput::Bytes(65536));
    group.bench_function("64KB", |b| {
        b.iter(|| codec::encode(black_box(&large_frame)))
    });

    group.finish();
}

/// Benchmark frame decoding.
fn bench_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode");

    // Small message
    let small_frame = Frame::publish("test:channel", vec![0u8; 64]);
    let small_encoded = codec::encode(&small_frame).unwrap();
    group.throughput(Throughput::Bytes(small_encoded.len() as u64));
    group.bench_function("64B", |b| {
        b.iter(|| codec::decode(black_box(&small_encoded)))
    });

    // Medium message
    let medium_frame = Frame::publish("test:channel", vec![0u8; 1024]);
    let medium_encoded = codec::encode(&medium_frame).unwrap();
    group.throughput(Throughput::Bytes(medium_encoded.len() as u64));
    group.bench_function("1KB", |b| {
        b.iter(|| codec::decode(black_box(&medium_encoded)))
    });

    // Large message
    let large_frame = Frame::publish("test:channel", vec![0u8; 65536]);
    let large_encoded = codec::encode(&large_frame).unwrap();
    group.throughput(Throughput::Bytes(large_encoded.len() as u64));
    group.bench_function("64KB", |b| {
        b.iter(|| codec::decode(black_box(&large_encoded)))
    });

    group.finish();
}

/// Benchmark router operations.
fn bench_router(c: &mut Criterion) {
    let mut group = c.benchmark_group("router");

    // Subscribe benchmark
    group.bench_function("subscribe", |b| {
        let router = Router::new();
        let mut i = 0u64;
        b.iter(|| {
            let channel = format!("channel:{}", i);
            let conn = format!("conn:{}", i);
            i += 1;
            let _ = router.subscribe(&conn, &channel);
        });
    });

    // Publish with 1 subscriber
    group.bench_function("publish_1_sub", |b| {
        let router = Router::new();
        let _rx = router.subscribe("conn-1", "test").unwrap();
        let message = Message::new("test", vec![0u8; 64]);

        b.iter(|| router.publish(black_box(message.clone())));
    });

    // Publish with 100 subscribers
    group.bench_function("publish_100_sub", |b| {
        let router = Router::new();
        let _rxs: Vec<_> = (0..100)
            .map(|i| router.subscribe(&format!("conn-{}", i), "test").unwrap())
            .collect();
        let message = Message::new("test", vec![0u8; 64]);

        b.iter(|| router.publish(black_box(message.clone())));
    });

    // Publish with 1000 subscribers
    group.bench_function("publish_1000_sub", |b| {
        let router = Router::new();
        let _rxs: Vec<_> = (0..1000)
            .map(|i| router.subscribe(&format!("conn-{}", i), "test").unwrap())
            .collect();
        let message = Message::new("test", vec![0u8; 64]);

        b.iter(|| router.publish(black_box(message.clone())));
    });

    group.finish();
}

/// Benchmark channel operations.
fn bench_channel(c: &mut Criterion) {
    use tenvis_pulse_core::Channel;

    let mut group = c.benchmark_group("channel");

    group.bench_function("subscribe", |b| {
        let mut channel = Channel::new("test");
        let mut i = 0u64;
        b.iter(|| {
            let conn = format!("conn-{}", i);
            i += 1;
            let _ = channel.subscribe(&conn);
        });
    });

    group.bench_function("publish", |b| {
        let mut channel = Channel::new("test");
        let _rx = channel.subscribe("conn-1");

        b.iter(|| channel.publish_payload(black_box(vec![0u8; 64])));
    });

    group.finish();
}

/// Benchmark fan-out scenarios.
fn bench_fanout(c: &mut Criterion) {
    let mut group = c.benchmark_group("fanout");

    for size in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let router = Router::new();
            let _rxs: Vec<_> = (0..size)
                .map(|i| {
                    router
                        .subscribe(&format!("conn-{}", i), "broadcast")
                        .unwrap()
                })
                .collect();
            let message = Message::new("broadcast", vec![0u8; 64]);

            b.iter(|| router.publish(black_box(message.clone())));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_encode,
    bench_decode,
    bench_router,
    bench_channel,
    bench_fanout,
);
criterion_main!(benches);
