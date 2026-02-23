use jpeg::JpegDecoder;
use jpeg_common::options::{DecoderOptions, SimdBackend};
use testutil::{save_pixels_as_pgm, save_pixels_as_ppm};

#[test_log::test]
#[ignore = "Progressive decoding is currently not supported"]
fn decode_progressive_jpeg400_scalar() {
    let data = include_bytes!("../../../testfiles/jpeg/w3/jpeg400jfif.prog.jpg");

    for backend in SimdBackend::iter() {
        let options = DecoderOptions::default().set_forced_simd_backend(Some(backend));
        let mut decoder = JpegDecoder::new_with_options(&data[..], options);
        let pixels = decoder.decode().unwrap();
        let info = decoder.info().unwrap();
        assert_eq!(info.width, 600);
        assert_eq!(info.height, 800);
        assert_eq!(info.precision, 8);
        assert_eq!(info.components, 1);

        // Write out the decoded pixels to a PGM file for visual verification
        save_pixels_as_pgm(
            &format!("jpeg400jfif.prog_{:?}.pgm", backend),
            &pixels,
            info.width,
            info.height,
            info.precision as usize,
        );
    }
}

#[test_log::test]
#[ignore = "Progressive decoding is currently not supported"]
fn decode_progressive_jpeg420_scalar() {
    let data = include_bytes!("../../../testfiles/jpeg/w3/jpeg420exif.prog.jpg");

    for backend in SimdBackend::iter() {
        let options = DecoderOptions::default().set_forced_simd_backend(Some(backend));
        let mut decoder = JpegDecoder::new_with_options(&data[..], options);
        let pixels = decoder.decode().unwrap();
        let info = decoder.info().unwrap();
        assert_eq!(info.width, 2048);
        assert_eq!(info.height, 1536);
        assert_eq!(info.precision, 8);
        assert_eq!(info.components, 3);

        // Write out the decoded pixels to a PPM file for visual verification
        save_pixels_as_ppm(
            &format!("jpeg420exif.prog_{:?}.ppm", backend),
            &pixels,
            info.width,
            info.height,
        );
    }
}

#[test_log::test]
#[ignore = "Progressive decoding is currently not supported"]
fn decode_progressive_jpeg422_scalar() {
    let data = include_bytes!("../../../testfiles/jpeg/w3/jpeg422jfif.prog.jpg");
    for backend in SimdBackend::iter() {
        let options = DecoderOptions::default().set_forced_simd_backend(Some(backend));
        let mut decoder = JpegDecoder::new_with_options(&data[..], options);
        let pixels = decoder.decode().unwrap();
        let info = decoder.info().unwrap();
        assert_eq!(info.width, 2048);
        assert_eq!(info.height, 1536);
        assert_eq!(info.precision, 8);
        assert_eq!(info.components, 3);

        // Write out the decoded pixels to a PPM file for visual verification
        save_pixels_as_ppm(
            &format!("jpeg422jfif.prog_{:?}.ppm", backend),
            &pixels,
            info.width,
            info.height,
        );
    }
}

#[test_log::test]
#[ignore = "Progressive decoding is currently not supported"]
fn decode_progressive_jpeg444_scalar() {
    let data = include_bytes!("../../../testfiles/jpeg/w3/jpeg444.prog.jpg");
    for backend in SimdBackend::iter() {
        let options = DecoderOptions::default().set_forced_simd_backend(Some(backend));
        let mut decoder = JpegDecoder::new_with_options(&data[..], options);
        let pixels = decoder.decode().unwrap();
        let info = decoder.info().unwrap();
        assert_eq!(info.width, 256);
        assert_eq!(info.height, 256);
        assert_eq!(info.precision, 8);
        assert_eq!(info.components, 3);

        // Write out the decoded pixels to a PPM file for visual verification
        save_pixels_as_ppm(
            &format!("jpeg444.prog_{:?}.ppm", backend),
            &pixels,
            info.width,
            info.height,
        );
    }
}
