use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use nexus_core::{native_add, native_checksum};

fn native_interoperability(c: &mut Criterion) {
    c.bench_function("native_add_i64_ffi", |b| {
        b.iter(|| native_add(black_box(20), black_box(22)).expect("native addition succeeds"))
    });
    c.bench_function("rust_add_i64_scalar", |b| {
        b.iter(|| black_box(black_box(20_i64).wrapping_add(black_box(22_i64))))
    });

    let mut group = c.benchmark_group("checksum_bytes");
    for size in [64_usize, 4 * 1024, 1024 * 1024] {
        let bytes = vec![0x5a_u8; size];
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("native_ffi", size), &bytes, |b, data| {
            b.iter(|| native_checksum(black_box(data)).expect("bounded native checksum"))
        });
        group.bench_with_input(BenchmarkId::new("rust_scalar", size), &bytes, |b, data| {
            b.iter(|| black_box(data).iter().fold(0_u64, |sum, byte| sum + u64::from(*byte)))
        });
    }
    group.finish();
}

criterion_group!(benches, native_interoperability);
criterion_main!(benches);
