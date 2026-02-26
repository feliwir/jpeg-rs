use jpeg::{JpegDecoder, JpegEncoder};
use jpeg_common::{
    color_space::ColorSpace,
    options::{EncoderOptions, SimdBackend},
};
use testutil::{compute_psnr, load_pixels_from_pgm, save_pixels_as_pgm};

#[test_log::test]
fn encode_extended_12bit() {
    let data = include_bytes!("../../../testfiles/pgm/mr1_jply.pgm");

    let (pixel_data, width, height, maxval) = load_pixels_from_pgm(data);
    assert_eq!(maxval, 4095);

    // Convert input from big-endian u16 bytes to a Vec<u16>
    let original_u16: Vec<u16> = pixel_data
        .chunks_exact(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();

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
            .set_precision(12)
            .set_forced_simd_backend(Some(backend));
        let mut encoder = JpegEncoder::new_with_options(&mut encoded, options).unwrap();
        encoder.encode(&pixel_data).unwrap();

        // Save as jpeg file
        std::fs::create_dir_all("out").unwrap();
        std::fs::write(&format!("out/mr1_jply_encoded_{}.jpg", backend), &encoded).unwrap();

        // Round-trip: decode the encoded JPEG and verify dimensions
        let mut decoder = JpegDecoder::new(&encoded[..]);
        let decoded = decoder.decode().unwrap();
        let info = decoder.info().unwrap();
        assert_eq!(info.width, width);
        assert_eq!(info.height, height);
        assert_eq!(info.components, 1);
        // 12-bit: decoder outputs 2 bytes per sample (little-endian u16)
        assert_eq!(decoded.len(), width * height * 2);

        // Convert decoded from little-endian u16 bytes to Vec<u16>
        let decoded_u16: Vec<u16> = decoded
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();

        // Check PSNR: lossy compression should produce reasonable quality
        let psnr = compute_psnr(&original_u16, &decoded_u16, 4095.0);
        log::info!(
            "Backend {backend}: encoded {} bytes, PSNR = {psnr:.2} dB",
            encoded.len()
        );
        assert!(
            psnr > 35.0,
            "PSNR too low for backend {backend}: {psnr:.2} dB (expected > 35 dB)"
        );

        // Write out the decoded pixels to a PGM file for visual verification
        save_pixels_as_pgm(
            &format!("out/mr1_jply_encoded_decoded_{}.pgm", backend),
            &decoded,
            width,
            height,
            info.precision as usize,
        );
    }
}
