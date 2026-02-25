use jpeg::JpegDecoder;
use jpeg_common::options::{DecoderOptions, SimdBackend};
use testutil::{save_pixels_as_pgm, save_pixels_as_ppm};

#[test_log::test]
fn decode_progressive_jpeg400() {
    let data = include_bytes!("../../../testfiles/jpeg/w3/jpeg400jfif.prog.jpg");

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
        decoder.decode_headers().unwrap();

        let (width, height, precision, components) = {
            let info = decoder.info().unwrap();
            (info.width, info.height, info.precision, info.components)
        };
        assert_eq!(width, 600);
        assert_eq!(height, 800);
        assert_eq!(precision, 8);
        assert_eq!(components, 1);

        let mut pixels = vec![0u8; decoder.required_buffer_size().unwrap()];
        let mut state = decoder.start_progressive().unwrap();

        while decoder.decode_next_scan(&mut state).unwrap() {
            decoder.reconstruct(&state, &mut pixels).unwrap();
            save_pixels_as_pgm(
                &format!(
                    "out/jpeg400jfif.prog_{}_scan{:02}.pgm",
                    backend,
                    state.scan_count()
                ),
                &pixels,
                width,
                height,
                precision as usize,
            );
        }
        // Final reconstruction
        decoder.reconstruct(&state, &mut pixels).unwrap();
        save_pixels_as_pgm(
            &format!("out/jpeg400jfif.prog_{}.pgm", backend),
            &pixels,
            width,
            height,
            precision as usize,
        );
    }
}

#[test_log::test]
fn decode_progressive_jpeg420() {
    let data = include_bytes!("../../../testfiles/jpeg/w3/jpeg420exif.prog.jpg");

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
        decoder.decode_headers().unwrap();

        let (width, height, precision, components) = {
            let info = decoder.info().unwrap();
            (info.width, info.height, info.precision, info.components)
        };
        assert_eq!(width, 2048);
        assert_eq!(height, 1536);
        assert_eq!(precision, 8);
        assert_eq!(components, 3);

        let mut pixels = vec![0u8; decoder.required_buffer_size().unwrap()];
        let mut state = decoder.start_progressive().unwrap();

        while decoder.decode_next_scan(&mut state).unwrap() {
            decoder.reconstruct(&state, &mut pixels).unwrap();
            save_pixels_as_ppm(
                &format!(
                    "out/jpeg420exif.prog_{}_scan{:02}.ppm",
                    backend,
                    state.scan_count()
                ),
                &pixels,
                width,
                height,
            );
        }
        decoder.reconstruct(&state, &mut pixels).unwrap();
        save_pixels_as_ppm(
            &format!("out/jpeg420exif.prog_{}.ppm", backend),
            &pixels,
            width,
            height,
        );
    }
}

#[test_log::test]
fn decode_progressive_jpeg422() {
    let data = include_bytes!("../../../testfiles/jpeg/w3/jpeg422jfif.prog.jpg");
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
        decoder.decode_headers().unwrap();

        let (width, height, precision, components) = {
            let info = decoder.info().unwrap();
            (info.width, info.height, info.precision, info.components)
        };
        assert_eq!(width, 2048);
        assert_eq!(height, 1536);
        assert_eq!(precision, 8);
        assert_eq!(components, 3);

        let mut pixels = vec![0u8; decoder.required_buffer_size().unwrap()];
        let mut state = decoder.start_progressive().unwrap();

        while decoder.decode_next_scan(&mut state).unwrap() {
            decoder.reconstruct(&state, &mut pixels).unwrap();
            save_pixels_as_ppm(
                &format!(
                    "out/jpeg422jfif.prog_{}_scan{:02}.ppm",
                    backend,
                    state.scan_count()
                ),
                &pixels,
                width,
                height,
            );
        }
        decoder.reconstruct(&state, &mut pixels).unwrap();
        save_pixels_as_ppm(
            &format!("out/jpeg422jfif.prog_{}.ppm", backend),
            &pixels,
            width,
            height,
        );
    }
}

#[test_log::test]
fn decode_progressive_jpeg444() {
    let data = include_bytes!("../../../testfiles/jpeg/w3/jpeg444.prog.jpg");
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
        decoder.decode_headers().unwrap();

        let (width, height, precision, components) = {
            let info = decoder.info().unwrap();
            (info.width, info.height, info.precision, info.components)
        };
        assert_eq!(width, 256);
        assert_eq!(height, 256);
        assert_eq!(precision, 8);
        assert_eq!(components, 3);

        let mut pixels = vec![0u8; decoder.required_buffer_size().unwrap()];
        let mut state = decoder.start_progressive().unwrap();

        while decoder.decode_next_scan(&mut state).unwrap() {
            decoder.reconstruct(&state, &mut pixels).unwrap();
            save_pixels_as_ppm(
                &format!(
                    "out/jpeg444.prog_{}_scan{:02}.ppm",
                    backend,
                    state.scan_count()
                ),
                &pixels,
                width,
                height,
            );
        }
        decoder.reconstruct(&state, &mut pixels).unwrap();
        save_pixels_as_ppm(
            &format!("out/jpeg444.prog_{}.ppm", backend),
            &pixels,
            width,
            height,
        );
    }
}
