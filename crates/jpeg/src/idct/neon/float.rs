// ── Inverse DCT ─────────────────────────────────────────────────────────────
// Intrinsics
use super::COS_TABLE;

/// Perform the 2D Inverse Discrete Cosine Transform on an 8×8 block.
///
/// This performs a column-row decomposition of the 2D IDCT,
/// which is more efficient than the direct O(N⁴) formula (specified in ITU-T T.81, Annex A)
/// while still being straightforward to understand.
#[target_feature(enable = "neon")]
pub fn idct<const PRECISION: u8>(block: &mut [i32; 64]) {
    let mut tmp = [0.0f32; 64];
    let mut output = [0.0f32; 64];

    let center = (1 << (PRECISION - 1)) as i32;
    let maxval = (1 << PRECISION) - 1;

    // Convert to f32
    let mut input = [0.0f32; 64];
    for i in 0..64 {
        input[i] = block[i] as f32;
    }

    // ----- Row pass -----
    for row in 0..8 {
        idct_1d(
            &input[row * 8..row * 8 + 8],
            &mut tmp[row * 8..row * 8 + 8],
            &COS_TABLE,
        )
    }

    // ----- Column pass -----
    for col in 0..8 {
        let mut col_in = [0.0f32; 8];
        let mut col_out = [0.0f32; 8];

        for row in 0..8 {
            col_in[row] = tmp[row * 8 + col];
        }

        idct_1d(&col_in, &mut col_out, &COS_TABLE);

        for row in 0..8 {
            output[row * 8 + col] = col_out[row];
        }
    }

    // ----- Final scaling, level shift, clamp -----
    for i in 0..64 {
        block[i] = (output[i].round() as i32 + center).clamp(0, maxval);
    }
}

// 1D IDCT that is applied on rows and columns in the AAN algorithm.
#[inline(always)]
fn idct_1d(input: &[f32], output: &mut [f32], cos_table: &[[f32; 8]; 8]) {
    unsafe {
        use std::arch::aarch64::*;

        let in_lo = vld1q_f32(input.as_ptr());
        let in_hi = vld1q_f32(input.as_ptr().add(4));

        for x in 0..8 {
            // Multiply-accumulate
            let cos_lo = vld1q_f32(cos_table[x].as_ptr());
            let cos_hi = vld1q_f32(cos_table[x].as_ptr().add(4));

            let mut sum = vmulq_f32(in_lo, cos_lo);
            sum = vfmaq_f32(sum, in_hi, cos_hi);

            output[x] = vaddvq_f32(sum) * 0.5;
        }
    }
}
