// ── Inverse DCT ─────────────────────────────────────────────────────────────
use super::{COS_TABLE_FIXED, FIX_BITS};

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
    for x in 0..8 {
        let mut sum: i64 = 0;

        for u in 0..8 {
            // input[u] * cu * cos
            sum += input[u] as i64 * cos_table[x][u] as i64;
        }

        // We multiplied by FIX_SCALE twice:
        // once for cu, once for cos_table
        //
        // So total scale = FIX_SCALE²
        //
        // Undo that:
        sum >>= FIX_BITS;

        // divide by 2 (because separable)
        sum >>= 1;

        output[x] = sum as i32;
    }
}
