//! Codec benchmarks for pulse-protocol.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use pulse_protocol::{codec, Frame};

fn bench_encode_small(c: &mut Criterion) {
    let frame = Frame::publish("test", vec![0u8; 64]);

    let mut group = c.benchmark_group("encode");
    group.throughput(Throughput::Bytes(64));
    group.bench_function("small_64B", |b| b.iter(|| codec::encode(black_box(&frame))));
    group.finish();
}

fn bench_decode_small(c: &mut Criterion) {
    let frame = Frame::publish("test", vec![0u8; 64]);
    let encoded = codec::encode(&frame).unwrap();

    let mut group = c.benchmark_group("decode");
    group.throughput(Throughput::Bytes(encoded.len() as u64));
    group.bench_function("small_64B", |b| {
        b.iter(|| codec::decode(black_box(&encoded)))
    });
    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let frame = Frame::publish("test:channel:room", vec![0u8; 256]);

    c.bench_function("roundtrip_256B", |b| {
        b.iter(|| {
            let encoded = codec::encode(black_box(&frame)).unwrap();
            codec::decode(black_box(&encoded)).unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_encode_small,
    bench_decode_small,
    bench_roundtrip
);
criterion_main!(benches);
