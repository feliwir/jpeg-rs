use criterion::{Criterion, criterion_group, criterion_main};
use jpeg::color_convert;

const NUM_PIXELS: usize = 64;

fn make_test_data() -> ([i32; NUM_PIXELS], [i32; NUM_PIXELS], [i32; NUM_PIXELS]) {
    let mut y = [0i32; NUM_PIXELS];
    let mut cb = [0i32; NUM_PIXELS];
    let mut cr = [0i32; NUM_PIXELS];
    for i in 0..NUM_PIXELS {
        y[i] = (i as i32 * 4) % 256;
        cb[i] = 100 + (i as i32 * 3) % 156;
        cr[i] = 50 + (i as i32 * 5) % 206;
    }
    (y, cb, cr)
}

fn bench_ycbcr_to_rgb(c: &mut Criterion) {
    let mut group = c.benchmark_group("YCbCr_to_RGB");
    let (y, cb, cr) = make_test_data();
    let mut rgb = [0u8; NUM_PIXELS * 3];

    group.bench_function("scalar", |b| {
        b.iter(|| {
            color_convert::scalar::ycbcr_to_rgb(&y, &cb, &cr, &mut rgb);
        })
    });

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    group.bench_function("sse", |b| {
        b.iter(|| unsafe {
            color_convert::sse::ycbcr_to_rgb(&y, &cb, &cr, &mut rgb);
        })
    });

    #[cfg(any(target_arch = "x86_64"))]
    group.bench_function("avx", |b| {
        b.iter(|| unsafe {
            color_convert::avx2::ycbcr_to_rgb(&y, &cb, &cr, &mut rgb);
        })
    });

    group.finish();
}

fn bench_rgb_to_ycbcr(c: &mut Criterion) {
    let mut group = c.benchmark_group("RGB_to_YCbCr");
    let mut rgb = [0u8; NUM_PIXELS * 3];
    for i in 0..NUM_PIXELS {
        rgb[i * 3] = (i as u16 * 4 % 256) as u8;
        rgb[i * 3 + 1] = 100 + (i as u8 * 3) % 156;
        rgb[i * 3 + 2] = 50 + (i as u8 * 5) % 206;
    }
    let mut ycbcr = [0u8; NUM_PIXELS * 3];

    group.bench_function("scalar", |b| {
        b.iter(|| {
            color_convert::scalar::rgb_to_ycbcr(&rgb, &mut ycbcr);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_ycbcr_to_rgb, bench_rgb_to_ycbcr);
criterion_main!(benches);
