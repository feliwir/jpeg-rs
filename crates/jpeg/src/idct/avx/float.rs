// ── Inverse DCT ─────────────────────────────────────────────────────────────
use super::COS_TABLE;

/// Perform the 2D Inverse Discrete Cosine Transform on an 8×8 block.
///
/// This performs a column-row decomposition of the 2D IDCT,
/// which is more efficient than the direct O(N⁴) formula (specified in ITU-T T.81, Annex A)
/// while still being straightforward to understand.
#[target_feature(enable = "avx512f")]
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
        );
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
        use std::arch::x86_64::*;
        // We compute all 8 outputs at once in one __m512 register.
        // Lower 8 lanes used, upper 8 unused.

        let mut acc = _mm512_setzero_ps();

        for u in 0..8 {
            // Broadcast input[u]
            let in_broadcast = _mm512_set1_ps(input[u]);

            // Load cosine column (8 floats)
            let cos_col = _mm256_loadu_ps(&[
                cos_table[0][u],
                cos_table[1][u],
                cos_table[2][u],
                cos_table[3][u],
                cos_table[4][u],
                cos_table[5][u],
                cos_table[6][u],
                cos_table[7][u],
            ] as *const f32);

            // Extend to 512 (upper lanes zero)
            let cos512 = _mm512_castps256_ps512(cos_col);

            // FMA accumulate
            acc = _mm512_fmadd_ps(in_broadcast, cos512, acc);
        }

        // Divide by 2
        acc = _mm512_mul_ps(acc, _mm512_set1_ps(0.5));

        // Store only lower 8 lanes
        let result256 = _mm512_castps512_ps256(acc);
        _mm256_storeu_ps(output.as_mut_ptr(), result256);
    }
}
