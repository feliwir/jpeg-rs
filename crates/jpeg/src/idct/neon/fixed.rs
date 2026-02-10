// ── Inverse DCT ─────────────────────────────────────────────────────────────

// Intrinsics
use super::COS_TABLE_FIXED;
use std::arch::aarch64::*;

/// Fixed-point scaling factor
const FIX_BITS: i32 = 14;

/// Perform the 2D Inverse Discrete Cosine Transform on an 8×8 block.
///
/// This performs a column-row decomposition of the 2D IDCT,
/// which is more efficient than the direct O(N⁴) formula (specified in ITU-T T.81, Annex A)
/// while still being straightforward to understand.
///
/// This version used fixed point arithmetic for better performance
pub fn idct_fixed<const PRECISION: u8>(block: &mut [i32; 64]) {
    let mut tmp = [0i32; 64];
    let mut output = [0i32; 64];

    let center = (1 << (PRECISION - 1)) as i32;
    let maxval = (1 << PRECISION) - 1;

    // ----- Row pass -----
    for row in 0..8 {
        idct_1d_fixed(
            &block[row * 8..row * 8 + 8],
            &mut tmp[row * 8..row * 8 + 8],
            &COS_TABLE_FIXED,
        )
    }

    // ----- Column pass -----
    for col in 0..8 {
        let mut col_in = [0i32; 8];
        let mut col_out = [0i32; 8];

        for row in 0..8 {
            col_in[row] = tmp[row * 8 + col];
        }

        idct_1d_fixed(&col_in, &mut col_out, &COS_TABLE_FIXED);

        for row in 0..8 {
            output[row * 8 + col] = col_out[row];
        }
    }

    // ----- Final scaling, level shift, clamp -----
    for i in 0..64 {
        block[i] = (output[i] + center).clamp(0, maxval);
    }
}

// 1D IDCT that is applied on rows and columns in the AAN algorithm.
#[inline(always)]
fn idct_1d_fixed(input: &[i32], output: &mut [i32], table: &[[i32; 8]; 8]) {
    unsafe {
        // Load input once
        let in_lo = vld1q_s32(input.as_ptr());
        let in_hi = vld1q_s32(input.as_ptr().add(4));

        for x in 0..8 {
            let row = table[x].as_ptr();

            let w_lo = vld1q_s32(row);
            let w_hi = vld1q_s32(row.add(4));

            // Multiply low 4 → 64-bit lanes
            let mul_lo = vmull_s32(vget_low_s32(in_lo), vget_low_s32(w_lo));

            let mul_hi = vmull_s32(vget_high_s32(in_lo), vget_high_s32(w_lo));

            let mul_lo2 = vmull_s32(vget_low_s32(in_hi), vget_low_s32(w_hi));

            let mul_hi2 = vmull_s32(vget_high_s32(in_hi), vget_high_s32(w_hi));

            // Sum 4 vectors of int64x2_t
            let sum1 = vaddq_s64(mul_lo, mul_hi);
            let sum2 = vaddq_s64(mul_lo2, mul_hi2);
            let sum_vec = vaddq_s64(sum1, sum2);

            // Horizontal add (2 lanes)
            let sum = vgetq_lane_s64(sum_vec, 0) + vgetq_lane_s64(sum_vec, 1);

            // Final scaling
            let mut result = sum >> FIX_BITS;
            result >>= 1;

            output[x] = result as i32;
        }
    }
}
