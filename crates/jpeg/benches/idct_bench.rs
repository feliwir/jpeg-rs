use criterion::{Criterion, criterion_group, criterion_main};
use jpeg::idct;

fn bench_idct(c: &mut Criterion) {
    let mut group = c.benchmark_group("IDCT");
    let mut block = [0i32; 64];
    // Fill with some sample data for a realistic test
    for i in 0..64 {
        block[i] = (i as i32) * 2 - 64;
    }

    group.bench_function("scalar", |b| {
        b.iter(|| {
            let mut block_copy = block;
            idct::scalar::idct::<8>(&mut block_copy);
        })
    });

    group.bench_function("scalar_fixed", |b| {
        b.iter(|| {
            let mut block_copy = block;
            idct::scalar::idct_fixed::<8>(&mut block_copy);
        })
    });

    #[cfg(target_arch = "aarch64")]
    group.bench_function("neon", |b| {
        b.iter(|| {
            let mut block_copy = block;
            idct::neon::idct::<8>(&mut block_copy);
        })
    });

    #[cfg(target_arch = "aarch64")]
    group.bench_function("neon_fixed", |b| {
        b.iter(|| {
            let mut block_copy = block;
            idct::neon::idct_fixed::<8>(&mut block_copy);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_idct);
criterion_main!(benches);
