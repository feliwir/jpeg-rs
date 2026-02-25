// ── Forward DCT (floating point) ─────────────────────────────────────────────
use super::COS_TABLE;

/// Perform the 2D forward Discrete Cosine Transform on an 8×8 block.
///
/// Input: 64 pixel values in row-major order (level-shifted by −128).
/// Output: 64 DCT coefficients in row-major order (rounded to i32).
pub fn fdct(block: &mut [i32; 64]) {
    let mut input = [0.0f32; 64];
    for i in 0..64 {
        input[i] = block[i] as f32;
    }

    let mut tmp = [0.0f32; 64];
    let mut output = [0.0f32; 64];

    // ----- Row pass -----
    for row in 0..8 {
        fdct_1d(
            &input[row * 8..row * 8 + 8],
            &mut tmp[row * 8..row * 8 + 8],
            &COS_TABLE,
        );
    }

    // ----- Column pass -----
    for col in 0..8 {
        let mut col_in = [0.0f32; 8];
        let mut col_out = [0.0f32; 8];

        for row in 0..8 {
            col_in[row] = tmp[row * 8 + col];
        }

        fdct_1d(&col_in, &mut col_out, &COS_TABLE);

        for row in 0..8 {
            output[row * 8 + col] = col_out[row];
        }
    }

    for i in 0..64 {
        block[i] = output[i].round() as i32;
    }
}

/// 1D forward DCT (floating point).
#[inline(always)]
fn fdct_1d(input: &[f32], output: &mut [f32], cos_table: &[[f32; 8]; 8]) {
    for u in 0..8 {
        let mut sum = 0.0f32;

        for x in 0..8 {
            sum = input[x].mul_add(cos_table[x][u], sum);
        }

        output[u] = sum / 2.0;
    }
}
