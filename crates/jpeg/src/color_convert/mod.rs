pub mod scalar;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod sse;

#[cfg(any(target_arch = "x86_64"))]
pub mod avx2;

use jpeg_common::options::SimdBackend;

use crate::component::MAX_SAMPLING_FACTOR;

// BT.601 full-range fixed-point coefficients (14-bit precision).
//
// Computed with MPFR for maximum accuracy.  Using i32 arithmetic
// so the compiler can auto-vectorize when appropriate.
pub(super) const Y_COEFF: i32 = 16384; //  1.0      × 2¹⁴
pub(super) const CR_R_COEFF: i32 = 22970; //  1.402    × 2¹⁴
pub(super) const CB_B_COEFF: i32 = 29032; //  1.772    × 2¹⁴
pub(super) const CR_G_COEFF: i32 = -11700; // -0.714136 × 2¹⁴
pub(super) const CB_G_COEFF: i32 = -5638; // -0.344136 × 2¹⁴
pub(super) const PRECISION: i32 = 14;
pub(super) const ROUND: i32 = (1 << (PRECISION - 1)) - 1;

/// Function pointer type for batch YCbCr→RGB conversion.
///
/// Converts `y.len()` pixels from YCbCr to interleaved RGB.
/// `cb` and `cr` must have at least as many elements as `y`.
/// `rgb` must have at least `y.len() * 3` bytes.
///
/// # Safety
/// SIMD variants require the corresponding CPU features to be available.
pub type YcbcrToRgbFn = unsafe fn(&[i32], &[i32], &[i32], &mut [u8]);

/// Select the best available YCbCr→RGB conversion function for the current
/// platform, optionally forced to a specific SIMD backend.
pub(crate) fn select_ycbcr_to_rgb_fn(forced_backend: Option<SimdBackend>) -> YcbcrToRgbFn {
    if let Some(backend) = forced_backend {
        match backend {
            SimdBackend::Scalar => return scalar::ycbcr_to_rgb,
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdBackend::Sse => return sse::ycbcr_to_rgb,
            #[cfg(any(target_arch = "x86_64"))]
            SimdBackend::Avx2 => return avx2::ycbcr_to_rgb,
            _ => return scalar::ycbcr_to_rgb,
        }
    }

    #[cfg(any(target_arch = "x86_64"))]
    if SimdBackend::is_supported(SimdBackend::Avx2) {
        return avx2::ycbcr_to_rgb;
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if SimdBackend::is_supported(SimdBackend::Sse) {
        return sse::ycbcr_to_rgb;
    }

    scalar::ycbcr_to_rgb
}

// ── Per-pixel component sampling ────────────────────────────────────────────

/// Maximum MCU width in pixels.
const MAX_MCU_WIDTH: usize = MAX_SAMPLING_FACTOR * 8;

/// Look up a pixel value from the decoded blocks of one component, handling
/// chroma subsampling via nearest-neighbor mapping.
///
/// `(px, py)` is the pixel position within the MCU in full-resolution
/// coordinates.  For subsampled components (e.g. 4:2:0 chroma with
/// `h_samples=1, v_samples=1` while `h_max=2, v_max=2`) the position is
/// scaled down to find the correct lower-resolution block and sample.
fn sample_component(
    blocks: &[[i32; 64]],
    h_samples: usize,
    v_samples: usize,
    h_max: usize,
    v_max: usize,
    px: usize,
    py: usize,
) -> i32 {
    // Map full-resolution pixel position to component coordinate space
    let cx = px * h_samples / h_max;
    let cy = py * v_samples / v_max;

    // Which 8×8 block, and which sample within that block?
    let block_idx = (cy / 8) * h_samples + (cx / 8);
    let sample_idx = (cy % 8) * 8 + (cx % 8);

    blocks[block_idx][sample_idx]
}

/// Write decoded MCU blocks to the output buffer, performing color conversion.
///
/// For grayscale images (1 component), writes the Y value directly.
/// For color images (3 components), gathers Y/Cb/Cr per row and converts
/// via the provided SIMD delegate function.
///
/// `h_samples` and `v_samples` are parallel slices of per-component sampling
/// factors; `h_max` / `v_max` are the maximum factors across all components.
#[allow(clippy::too_many_arguments)]
pub(crate) fn write_mcu_pixels(
    ycbcr_to_rgb_fn: YcbcrToRgbFn,
    mcu_blocks: &[Vec<[i32; 64]>],
    h_samples: &[usize],
    v_samples: &[usize],
    h_max: usize,
    v_max: usize,
    mcu_w: usize,
    mcu_h: usize,
    mcu_col: usize,
    mcu_row: usize,
    img_w: usize,
    img_h: usize,
    num_components: usize,
    bytes_per_pixel: usize,
    output: &mut [u8],
) {
    if num_components == 1 {
        // Grayscale — write Y values directly
        for py in 0..mcu_h {
            let abs_y = mcu_row * mcu_h + py;
            if abs_y >= img_h {
                break;
            }

            let abs_x_start = mcu_col * mcu_w;
            let valid_w = mcu_w.min(img_w.saturating_sub(abs_x_start));
            let out_start = (abs_y * img_w + abs_x_start) * bytes_per_pixel;

            for px in 0..valid_w {
                let val = sample_component(
                    &mcu_blocks[0],
                    h_samples[0],
                    v_samples[0],
                    h_max,
                    v_max,
                    px,
                    py,
                );

                if bytes_per_pixel == 1 {
                    output[out_start + px] = val as u8;
                } else {
                    // >8-bit: store as little-endian u16
                    let dst = out_start + px * bytes_per_pixel;
                    let bytes = (val as u16).to_le_bytes();
                    output[dst] = bytes[0];
                    output[dst + 1] = bytes[1];
                }
            }
        }
    } else {
        // YCbCr → RGB via SIMD delegate
        let mut y_row = [0i32; MAX_MCU_WIDTH];
        let mut cb_row = [0i32; MAX_MCU_WIDTH];
        let mut cr_row = [0i32; MAX_MCU_WIDTH];
        let mut rgb_row = [0u8; MAX_MCU_WIDTH * 3];

        for py in 0..mcu_h {
            let abs_y = mcu_row * mcu_h + py;
            if abs_y >= img_h {
                break;
            }

            let abs_x_start = mcu_col * mcu_w;
            let valid_w = mcu_w.min(img_w.saturating_sub(abs_x_start));
            if valid_w == 0 {
                continue;
            }

            // Gather Y, Cb, Cr values for this row
            for px in 0..valid_w {
                y_row[px] = sample_component(
                    &mcu_blocks[0],
                    h_samples[0],
                    v_samples[0],
                    h_max,
                    v_max,
                    px,
                    py,
                );
                cb_row[px] = sample_component(
                    &mcu_blocks[1],
                    h_samples[1],
                    v_samples[1],
                    h_max,
                    v_max,
                    px,
                    py,
                );
                cr_row[px] = sample_component(
                    &mcu_blocks[2],
                    h_samples[2],
                    v_samples[2],
                    h_max,
                    v_max,
                    px,
                    py,
                );
            }

            // Batch YCbCr→RGB conversion
            unsafe {
                (ycbcr_to_rgb_fn)(
                    &y_row[..valid_w],
                    &cb_row[..valid_w],
                    &cr_row[..valid_w],
                    &mut rgb_row[..valid_w * 3],
                );
            }

            // Write to output buffer
            let out_start = (abs_y * img_w + abs_x_start) * bytes_per_pixel;
            if bytes_per_pixel == 3 {
                output[out_start..out_start + valid_w * 3].copy_from_slice(&rgb_row[..valid_w * 3]);
            } else {
                for px in 0..valid_w {
                    let dst = out_start + px * bytes_per_pixel;
                    let src = px * 3;
                    output[dst] = rgb_row[src];
                    output[dst + 1] = rgb_row[src + 1];
                    output[dst + 2] = rgb_row[src + 2];
                }
            }
        }
    }
}
