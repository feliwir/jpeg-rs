use criterion::{Criterion, criterion_group, criterion_main};
use jpeg::dct;

fn bench_fdct(c: &mut Criterion) {
    let mut group = c.benchmark_group("FDCT");
    let mut block = [0i32; 64];
    // Fill with some sample data for a realistic test
    for i in 0..64 {
        block[i] = (i as i32) * 2 - 64;
    }

    group.bench_function("scalar", |b| {
        b.iter(|| {
            let mut block_copy = block;
            dct::scalar::fdct(&mut block_copy);
        })
    });

    group.bench_function("scalar_fixed", |b| {
        b.iter(|| {
            let mut block_copy = block;
            dct::scalar::fdct_fixed(&mut block_copy);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_fdct);
criterion_main!(benches);
