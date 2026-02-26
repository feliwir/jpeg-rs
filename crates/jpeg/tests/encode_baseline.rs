use jpeg::{JpegDecoder, JpegEncoder};
use jpeg_common::{
    color_space::ColorSpace,
    options::{EncoderOptions, SimdBackend},
};
use testutil::{compute_psnr, load_pixels_from_pgm, load_pixels_from_ppm};

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
        let psnr = compute_psnr(&pixel_data, &decoded, 255.0);
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

#[test_log::test]
fn encode_baseline_jpeg420() {
    let data = include_bytes!("../../../testfiles/ppm/jpeg420.ppm");

    let (pixel_data, width, height, maxval) = load_pixels_from_ppm(data);
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
        let options = EncoderOptions::new(width, height, ColorSpace::RGB)
            .set_forced_simd_backend(Some(backend));
        let mut encoder = JpegEncoder::new_with_options(&mut encoded, options).unwrap();
        encoder.encode(&pixel_data).unwrap();

        // Save as jpeg file
        std::fs::create_dir_all("out").unwrap();
        std::fs::write(&format!("out/jpeg420_encoded_{}.jpg", backend), &encoded).unwrap();

        // Round-trip: decode the encoded JPEG and verify dimensions
        let mut decoder = JpegDecoder::new(&encoded[..]);
        let decoded = decoder.decode().unwrap();
        let info = decoder.info().unwrap();
        assert_eq!(info.width, width);
        assert_eq!(info.height, height);
        assert_eq!(info.components, 3);
        assert_eq!(decoded.len(), width * height * 3);

        // Check PSNR: lossy compression should produce reasonable quality
        let psnr = compute_psnr(&pixel_data, &decoded, 255.0);
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

#[test_log::test]
fn encode_baseline_jpeg444() {
    let data = include_bytes!("../../../testfiles/ppm/jpeg444.ppm");

    let (pixel_data, width, height, maxval) = load_pixels_from_ppm(data);
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
        let options = EncoderOptions::new(width, height, ColorSpace::RGB)
            .set_forced_simd_backend(Some(backend));
        let mut encoder = JpegEncoder::new_with_options(&mut encoded, options).unwrap();
        encoder.encode(&pixel_data).unwrap();

        // Save as jpeg file
        std::fs::create_dir_all("out").unwrap();
        std::fs::write(&format!("out/jpeg444_encoded_{}.jpg", backend), &encoded).unwrap();

        // Round-trip: decode the encoded JPEG and verify dimensions
        let mut decoder = JpegDecoder::new(&encoded[..]);
        let decoded = decoder.decode().unwrap();
        let info = decoder.info().unwrap();
        assert_eq!(info.width, width);
        assert_eq!(info.height, height);
        assert_eq!(info.components, 3);
        assert_eq!(decoded.len(), width * height * 3);

        // Check PSNR: lossy compression should produce reasonable quality
        let psnr = compute_psnr(&pixel_data, &decoded, 255.0);
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
