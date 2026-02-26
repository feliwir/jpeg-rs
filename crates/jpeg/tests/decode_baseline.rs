use jpeg::JpegDecoder;
use jpeg_common::{
    color_space::ColorSpace,
    options::{DecoderOptions, SimdBackend},
};
use testutil::{save_pixels_as_pgm, save_pixels_as_ppm};

#[test_log::test]
fn decode_baseline_jpeg400() {
    let data = include_bytes!("../../../testfiles/jpeg/w3/jpeg400jfif.jpg");

    for backend in SimdBackend::iter() {
        if !backend.is_supported() {
            log::warn!(
                "Skipping {} backend: not supported on this platform",
                backend
            );
            continue;
        }

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
            &format!("out/jpeg400jfif_{}.pgm", backend),
            &pixels,
            info.width,
            info.height,
            info.precision as usize,
        );
    }
}

#[test_log::test]
fn decode_baseline_jpeg420() {
    let data = include_bytes!("../../../testfiles/jpeg/w3/jpeg420exif.jpg");

    for backend in SimdBackend::iter() {
        if !backend.is_supported() {
            log::warn!(
                "Skipping {} backend: not supported on this platform",
                backend
            );
            continue;
        }

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
            &format!("out/jpeg420exif_{}.ppm", backend),
            &pixels,
            info.width,
            info.height,
        );
    }
}

#[test_log::test]
fn decode_baseline_jpeg420_as_grayscale() {
    let data = include_bytes!("../../../testfiles/jpeg/w3/jpeg420exif.jpg");

    for backend in SimdBackend::iter() {
        if !backend.is_supported() {
            log::warn!(
                "Skipping {} backend: not supported on this platform",
                backend
            );
            continue;
        }

        let options = DecoderOptions::default()
            .set_forced_simd_backend(Some(backend))
            .set_out_colorspace(Some(ColorSpace::Grayscale));
        let mut decoder = JpegDecoder::new_with_options(&data[..], options);
        let pixels = decoder.decode().unwrap();
        let info = decoder.info().unwrap();
        assert_eq!(info.width, 2048);
        assert_eq!(info.height, 1536);
        assert_eq!(info.precision, 8);
        assert_eq!(info.components, 1);

        // Write out the decoded pixels to a PGM file for visual verification
        save_pixels_as_pgm(
            &format!("out/jpeg420exif_grayscale_{}.pgm", backend),
            &pixels,
            info.width,
            info.height,
            info.precision as usize,
        );
    }
}

#[test_log::test]
fn decode_baseline_jpeg422() {
    let data = include_bytes!("../../../testfiles/jpeg/w3/jpeg422jfif.jpg");
    for backend in SimdBackend::iter() {
        if !backend.is_supported() {
            log::warn!(
                "Skipping {} backend: not supported on this platform",
                backend
            );
            continue;
        }

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
            &format!("out/jpeg422jfif_{}.ppm", backend),
            &pixels,
            info.width,
            info.height,
        );
    }
}

#[test_log::test]
fn decode_baseline_jpeg444() {
    let data = include_bytes!("../../../testfiles/jpeg/w3/jpeg444.jpg");
    for backend in SimdBackend::iter() {
        if !backend.is_supported() {
            log::warn!(
                "Skipping {} backend: not supported on this platform",
                backend
            );
            continue;
        }

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
            &format!("out/jpeg444_{}.ppm", backend),
            &pixels,
            info.width,
            info.height,
        );
    }
}
