// ── Forward DCT (fixed-point) ────────────────────────────────────────────────
use super::COS_TABLE_FIXED;

/// Fixed-point scaling factor (must match the IDCT table precision).
const FIX_BITS: i32 = 14;

/// Perform the 2D forward Discrete Cosine Transform on an 8×8 block.
///
/// Input: 64 pixel values in row-major order (level-shifted by −128).
/// Output: 64 DCT coefficients in row-major order.
///
/// This uses the same separable column-row decomposition as the IDCT,
/// but the summation runs over spatial indices rather than frequency indices.
pub fn fdct_fixed(block: &mut [i32; 64]) {
    let mut tmp = [0i32; 64];
    let mut output = [0i32; 64];

    // ----- Row pass: DCT each row -----
    for row in 0..8 {
        fdct_1d_fixed(
            &block[row * 8..row * 8 + 8],
            &mut tmp[row * 8..row * 8 + 8],
            &COS_TABLE_FIXED,
        );
    }

    // ----- Column pass: DCT each column -----
    for col in 0..8 {
        let mut col_in = [0i32; 8];
        let mut col_out = [0i32; 8];

        for row in 0..8 {
            col_in[row] = tmp[row * 8 + col];
        }

        fdct_1d_fixed(&col_in, &mut col_out, &COS_TABLE_FIXED);

        for row in 0..8 {
            output[row * 8 + col] = col_out[row];
        }
    }

    *block = output;
}

/// 1D forward DCT using fixed-point arithmetic.
///
/// For the forward DCT the formula is:
///   F(u) = (cu / 2) · Σ_{x=0..7} f(x) · cos((2x+1)·u·π/16)
///
/// The cos table is indexed as `cos_table[x][u]` and already includes
/// the cu normalization, so we just multiply and accumulate.
#[inline(always)]
fn fdct_1d_fixed(input: &[i32], output: &mut [i32], cos_table: &[[i32; 8]; 8]) {
    for u in 0..8 {
        let mut sum: i64 = 0;

        for x in 0..8 {
            // cos_table[x][u] already contains cu * cos(...)
            sum += input[x] as i64 * cos_table[x][u] as i64;
        }

        // Undo the fixed-point scaling (one factor of 2^FIX_BITS)
        sum >>= FIX_BITS;

        // Divide by 2 (from the 1/2 normalization in the separable form)
        sum >>= 1;

        output[u] = sum as i32;
    }
}
