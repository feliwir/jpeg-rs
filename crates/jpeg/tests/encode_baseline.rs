use jpeg::{JpegDecoder, JpegEncoder};
use jpeg_common::{
    color_space::ColorSpace,
    options::{EncoderOptions, SimdBackend},
};
use testutil::load_pixels_from_pgm;

#[test_log::test]
fn encode_baseline_jpeg400() {
    let data = include_bytes!("../../../testfiles/pgm/jpeg400jfif.pgm");

    let (pixel_data, width, height, maxval) = load_pixels_from_pgm(data);
    assert_eq!(maxval, 255);

    for backend in SimdBackend::iter() {
        if !backend.is_supported() {
            log::warn!(
                "Skipping {} backend: not supported on this platform",
                backend
            );
            continue;
        }

        let mut encoded = Vec::new();
        let options = EncoderOptions::new(width, height, ColorSpace::Grayscale)
            .set_forced_simd_backend(Some(backend));
        let mut encoder = JpegEncoder::new_with_options(&mut encoded, options).unwrap();
        encoder.encode(&pixel_data).unwrap();

        // Save as jpeg file
        std::fs::create_dir_all("out").unwrap();
        std::fs::write(
            &format!("out/jpeg400jfif_encoded_{}.jpg", backend),
            &encoded,
        )
        .unwrap();

        // Round-trip: decode the encoded JPEG and verify dimensions
        let mut decoder = JpegDecoder::new(&encoded[..]);
        let decoded = decoder.decode().unwrap();
        let info = decoder.info().unwrap();
        assert_eq!(info.width, width);
        assert_eq!(info.height, height);
        assert_eq!(info.components, 1);
        assert_eq!(decoded.len(), width * height);

        // Check PSNR: lossy compression should produce reasonable quality
        let psnr = compute_psnr(&pixel_data, &decoded);
        log::info!(
            "Backend {backend}: encoded {} bytes, PSNR = {psnr:.2} dB",
            encoded.len()
        );
        assert!(
            psnr > 35.0,
            "PSNR too low for backend {backend}: {psnr:.2} dB (expected > 35 dB)"
        );
    }
}

/// Compute the Peak Signal-to-Noise Ratio between two images.
fn compute_psnr(original: &[u8], decoded: &[u8]) -> f64 {
    assert_eq!(original.len(), decoded.len());
    let mse: f64 = original
        .iter()
        .zip(decoded.iter())
        .map(|(&a, &b)| {
            let diff = a as f64 - b as f64;
            diff * diff
        })
        .sum::<f64>()
        / original.len() as f64;

    if mse == 0.0 {
        return f64::INFINITY;
    }
    10.0 * (255.0 * 255.0 / mse).log10()
}
