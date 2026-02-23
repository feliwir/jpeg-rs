use super::COS_TABLE;

/// Perform the 2D Inverse Discrete Cosine Transform on an 8×8 block.
///
/// This performs a column-row decomposition of the 2D IDCT,
/// which is more efficient than the direct O(N⁴) formula (specified in ITU-T T.81, Annex A)
/// while still being straightforward to understand.
#[target_feature(enable = "sse4.1")]
pub fn idct<const PRECISION: u8>(block: &mut [i32; 64]) {
    let mut tmp = [0.0f32; 64];
    let mut output = [0.0f32; 64];

    let center = (1 << (PRECISION - 1)) as i32;
    let maxval = (1 << PRECISION) - 1;

    let mut input = [0.0f32; 64];

    // Convert to f32
    for i in 0..64 {
        input[i] = block[i] as f32;
    }

    // ----- Row pass -----
    for row in 0..8 {
        idct_1d(&input[row * 8..row * 8 + 8], &mut tmp[row * 8..row * 8 + 8]);
    }

    // ----- Column pass -----
    for col in 0..8 {
        let mut col_in = [0.0f32; 8];
        let mut col_out = [0.0f32; 8];

        for row in 0..8 {
            col_in[row] = tmp[row * 8 + col];
        }

        idct_1d(&col_in, &mut col_out);

        for row in 0..8 {
            output[row * 8 + col] = col_out[row];
        }
    }

    // ----- Final scaling -----
    for i in 0..64 {
        block[i] = (output[i].round() as i32 + center).clamp(0, maxval);
    }
}

#[inline(always)]
fn idct_1d(input: &[f32], output: &mut [f32]) {
    unsafe {
        use std::arch::x86_64::*;

        // Load input once
        let in_lo = _mm_loadu_ps(input.as_ptr());
        let in_hi = _mm_loadu_ps(input.as_ptr().add(4));

        for x in 0..8 {
            let cos_ptr = COS_TABLE[x].as_ptr();

            let cos_lo = _mm_loadu_ps(cos_ptr);
            let cos_hi = _mm_loadu_ps(cos_ptr.add(4));

            let mul_lo = _mm_mul_ps(in_lo, cos_lo);
            let mul_hi = _mm_mul_ps(in_hi, cos_hi);

            let sum = _mm_add_ps(mul_lo, mul_hi);

            // Horizontal sum
            let sum1 = _mm_hadd_ps(sum, sum);
            let sum2 = _mm_hadd_ps(sum1, sum1);

            let result = _mm_cvtss_f32(sum2);

            output[x] = result * 0.5;
        }
    }
}
