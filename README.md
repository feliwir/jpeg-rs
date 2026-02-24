# jpeg-rs

A pure-Rust workspace of JPEG family codecs, designed for **full specification coverage** across all JPEG transfer syntaxes used in medical imaging (DICOM).

## Motivation

Most existing JPEG libraries target the web: they handle baseline 8-bit 4:2:0 and call it a day. Medical imaging (DICOM) demands far more — extended sequential with 12- and 16-bit precision, progressive refinement, lossless modes, and an entire family of related standards (JPEG-LS, JPEG 2000, JPEG XL). Few decoders support all of these, and even fewer expose them behind a single, consistent interface.

**jpeg-rs** aims to fill that gap: one workspace, one API surface, full-spec coverage.

## Crates

| Crate | Description | Decoder | Encoder |
|-------|-------------|---------|---------|
| `jpeg` | Baseline, extended sequential & progressive DCT (ITU-T T.81) | Active | Planned |
| `jpeg-ls` | JPEG-LS lossless & near-lossless (ITU-T T.87) | Planned | Planned |
| `jpeg-2000` | JPEG 2000 (ITU-T T.800) | Planned | Planned |
| `jpeg-xl` | JPEG XL (ISO 18181) | Planned | Planned |
| `jpeg-common` | Shared types: color spaces, decoder options, SIMD backend selection | — | — |
| `testutil` | PGM/PPM writers and DICOM test-file helpers | — | — |

## Features

### JPEG decoder (`jpeg` crate)

- **Baseline DCT** (SOF0) — 8-bit, all chroma sub-samplings (4:4:4, 4:2:2, 4:2:0, 4:0:0)
- **Extended sequential DCT** (SOF1) — 8-bit and 12-bit precision
- **Progressive DCT** (SOF2) — full DC/AC spectral selection and successive approximation, with a pull-based API for incremental rendering
- **SIMD-accelerated IDCT** — Scalar, SSE, AVX, and NEON backends (fixed-point and floating-point)
- **SIMD-accelerated YCbCr→RGB** — Scalar, SSE, and AVX backends with BT.601 full-range fixed-point coefficients
- **Restart marker** support
- **JFIF & EXIF** metadata tolerance

### Progressive pull API

```rust
use jpeg::{JpegDecoder, ProgressiveState};

let mut decoder = JpegDecoder::new(&data[..]);
decoder.decode_headers()?;

let mut pixels = vec![0u8; decoder.required_buffer_size().unwrap()];
let mut state = decoder.start_progressive()?;

while decoder.decode_next_scan(&mut state)? {
    decoder.reconstruct(&state, &mut pixels)?;
    // pixels now contains the best image available so far
}
```

## Building

```sh
cargo build
cargo test
```

## License

See individual crate manifests for license information.
