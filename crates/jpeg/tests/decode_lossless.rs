use jpeg::JpegDecoder;
use jpeg_common::options::{DecoderOptions, SimdBackend};
use testutil::{extract_pixel_data_from_dicom, save_pixels_as_pgm};

#[test_log::test]
fn decode_lossless_16bit_ct1() {
    let data = include_bytes!("../../../testfiles/dicom/JPEG/IMAGES/JPLL/CT1_JPLL.dcm");
    let pixel_data = extract_pixel_data_from_dicom(&data[..], 0);

    for backend in SimdBackend::iter() {
        if !backend.is_supported() {
            log::warn!(
                "Skipping {:?} backend: not supported on this platform",
                backend
            );
            continue;
        }

        let options = DecoderOptions::default().set_forced_simd_backend(Some(backend));
        let mut decoder = JpegDecoder::new_with_options(&pixel_data[..], options);

        let pixels = decoder.decode().unwrap();
        let info = decoder.info().unwrap();
        assert_eq!(info.width, 512);
        assert_eq!(info.height, 512);
        assert_eq!(info.precision, 16);
        assert_eq!(info.components, 1);

        // Write out the decoded pixels to a PGM file for visual verification
        save_pixels_as_pgm(
            &format!("ct1_jpll_{:?}.pgm", backend),
            &pixels,
            info.width,
            info.height,
            info.precision as usize,
        );
    }
}

#[test_log::test]
#[ignore = "This test currently fails due to a bug in the lossless decoder. We will fix this soon and re-enable the test."]
fn decode_lossless_16bit_mr1() {
    let data = include_bytes!("../../../testfiles/dicom/JPEG/IMAGES/JPLL/MR1_JPLL.dcm");
    let pixel_data = extract_pixel_data_from_dicom(&data[..], 0);

    for backend in SimdBackend::iter() {
        if !backend.is_supported() {
            log::warn!(
                "Skipping {:?} backend: not supported on this platform",
                backend
            );
            continue;
        }

        let options = DecoderOptions::default().set_forced_simd_backend(Some(backend));
        let mut decoder = JpegDecoder::new_with_options(&pixel_data[..], options);

        let pixels = decoder.decode().unwrap();
        let info = decoder.info().unwrap();
        assert_eq!(info.width, 512);
        assert_eq!(info.height, 512);
        assert_eq!(info.precision, 16);
        assert_eq!(info.components, 1);

        // Write out the decoded pixels to a PGM file for visual verification
        save_pixels_as_pgm(
            &format!("mr1_jpll_{:?}.pgm", backend),
            &pixels,
            info.width,
            info.height,
            info.precision as usize,
        );
    }
}

#[test_log::test]
fn decode_lossless_12bit() {
    let data = include_bytes!("../../../testfiles/dicom/JPEG/IMAGES/JPLL/MR2_JPLL.dcm");
    let pixel_data = extract_pixel_data_from_dicom(&data[..], 0);

    for backend in SimdBackend::iter() {
        if !backend.is_supported() {
            log::warn!(
                "Skipping {:?} backend: not supported on this platform",
                backend
            );
            continue;
        }

        let options = DecoderOptions::default().set_forced_simd_backend(Some(backend));
        let mut decoder = JpegDecoder::new_with_options(&pixel_data[..], options);

        let pixels = decoder.decode().unwrap();
        let info = decoder.info().unwrap();
        assert_eq!(info.width, 1024);
        assert_eq!(info.height, 1024);
        assert_eq!(info.precision, 12);
        assert_eq!(info.components, 1);

        // Write out the decoded pixels to a PGM file for visual verification
        save_pixels_as_pgm(
            &format!("mr2_jpll_{:?}.pgm", backend),
            &pixels,
            info.width,
            info.height,
            info.precision as usize,
        );
    }
}