// ── Inverse DCT ─────────────────────────────────────────────────────────────
use super::{COS_TABLE_FIXED, FIX_BITS};

/// Perform the 2D Inverse Discrete Cosine Transform on an 8×8 block.
///
/// This performs a column-row decomposition of the 2D IDCT,
/// which is more efficient than the direct O(N⁴) formula (specified in ITU-T T.81, Annex A)
/// while still being straightforward to understand.
///
/// This version used fixed point arithmetic for better performance
#[target_feature(enable = "sse4.1")]
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
        );
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
fn idct_1d_fixed(input: &[i32], output: &mut [i32], cos_table: &[[i32; 8]; 8]) {
    unsafe {
        use std::arch::x86_64::*;

        // Load input once
        let in0 = _mm_loadu_si128(input.as_ptr() as *const __m128i);
        let in1 = _mm_loadu_si128(input.as_ptr().add(4) as *const __m128i);

        for x in 0..8 {
            let cos_ptr = cos_table[x].as_ptr();

            let c0 = _mm_loadu_si128(cos_ptr as *const __m128i);
            let c1 = _mm_loadu_si128(cos_ptr.add(4) as *const __m128i);

            // ---- Even indices (0,2,4,6) ----
            let mul0 = _mm_mul_epi32(in0, c0);
            let mul1 = _mm_mul_epi32(in1, c1);

            let sum_even = _mm_add_epi64(mul0, mul1);

            // ---- Odd indices (1,3,5,7) ----
            let in0_shift = _mm_srli_si128(in0, 4);
            let in1_shift = _mm_srli_si128(in1, 4);
            let c0_shift = _mm_srli_si128(c0, 4);
            let c1_shift = _mm_srli_si128(c1, 4);

            let mul0_odd = _mm_mul_epi32(in0_shift, c0_shift);
            let mul1_odd = _mm_mul_epi32(in1_shift, c1_shift);

            let sum_odd = _mm_add_epi64(mul0_odd, mul1_odd);

            // Combine even + odd
            let sum = _mm_add_epi64(sum_even, sum_odd);

            // Horizontal add (2 lanes)
            let sum_hi = _mm_unpackhi_epi64(sum, sum);
            let sum_total = _mm_add_epi64(sum, sum_hi);

            let mut result = _mm_cvtsi128_si64(sum_total);

            // Undo scaling
            result >>= FIX_BITS;
            result >>= 1;

            output[x] = result as i32;
        }
    }
}
