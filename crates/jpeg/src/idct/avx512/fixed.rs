// ── Inverse DCT ─────────────────────────────────────────────────────────────
use super::{COS_TABLE_FIXED, FIX_BITS};

/// Perform the 2D Inverse Discrete Cosine Transform on an 8×8 block.
///
/// This performs a column-row decomposition of the 2D IDCT,
/// which is more efficient than the direct O(N⁴) formula (specified in ITU-T T.81, Annex A)
/// while still being straightforward to understand.
///
/// This version used fixed point arithmetic for better performance
#[target_feature(enable = "avx512f,avx512dq")]
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

#[inline(always)]
fn idct_1d_fixed(input: &[i32], output: &mut [i32], cos_table: &[[i32; 8]; 8]) {
    unsafe {
        use std::arch::x86_64::*;

        // Accumulate each output as i64
        let mut sum = [_mm512_setzero_si512(); 1]; // We'll only need 1 register for 8 outputs

        for u in 0..8 {
            // Broadcast input[u] to 64-bit lanes
            let in_val = _mm512_set1_epi64(input[u] as i64);

            // Load cosine column: cos[0..7][u] -> i64
            let cos_col: [i64; 8] = [
                cos_table[0][u] as i64,
                cos_table[1][u] as i64,
                cos_table[2][u] as i64,
                cos_table[3][u] as i64,
                cos_table[4][u] as i64,
                cos_table[5][u] as i64,
                cos_table[6][u] as i64,
                cos_table[7][u] as i64,
            ];
            let cos_vec = _mm512_loadu_si512(cos_col.as_ptr() as *const __m512i);

            // Multiply: i64 lanes
            let prod = _mm512_mullo_epi64(in_val, cos_vec);

            // Accumulate
            sum[0] = _mm512_add_epi64(sum[0], prod);
        }

        // Shift right by FIX_BITS + 1 (divide by 2)
        let shift = FIX_BITS + 1;
        let mut sum_shifted: [i64; 8] = [0; 8];
        _mm512_storeu_si512(sum_shifted.as_mut_ptr() as *mut __m512i, sum[0]);
        for i in 0..8 {
            output[i] = (sum_shifted[i] >> shift) as i32;
        }
    }
}
