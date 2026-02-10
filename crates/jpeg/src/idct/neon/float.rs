// ── Inverse DCT ─────────────────────────────────────────────────────────────
// Intrinsics
use super::COS_TABLE;
use std::arch::aarch64::*;

/// Preloaded cosine tables for NEON.
struct CosTableNeon {
    lo: [float32x4_t; 8],
    hi: [float32x4_t; 8],
}

#[inline(always)]
fn load_cos_table(cos_table: &[[f32; 8]; 8]) -> CosTableNeon {
    unsafe {
        let mut lo = [vdupq_n_f32(0.0); 8];
        let mut hi = [vdupq_n_f32(0.0); 8];

        for i in 0..8 {
            lo[i] = vld1q_f32(cos_table[i].as_ptr());
            hi[i] = vld1q_f32(cos_table[i].as_ptr().add(4));
        }

        CosTableNeon { lo, hi }
    }
}

/// Perform the 2D Inverse Discrete Cosine Transform on an 8×8 block.
///
/// This uses the direct O(N⁴) formula from the JPEG specification
/// (ITU-T T.81, Annex A) for maximum clarity:
///
/// ```text
///   f(x,y) = (1/4) × Σ_u Σ_v  C(u) · C(v) · F(u,v)
///            × cos((2x+1)·u·π/16) × cos((2y+1)·v·π/16)
/// ```
///
/// where `C(0) = 1/√2` and `C(k) = 1` for `k > 0`.
///
/// After the transform, values are level-shifted by +128 and clamped to [0, 255].
///
/// A faster implementation (e.g. AAN or row-column decomposition) can be
/// substituted here without changing anything else.
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

    let cos_table_neon = load_cos_table(&COS_TABLE);

    // ----- Row pass -----
    for row in 0..8 {
        idct_1d(
            &input[row * 8..row * 8 + 8],
            &mut tmp[row * 8..row * 8 + 8],
            &cos_table_neon,
        )
    }

    // ----- Column pass -----
    for col in 0..8 {
        let mut col_in = [0.0f32; 8];
        let mut col_out = [0.0f32; 8];

        for row in 0..8 {
            col_in[row] = tmp[row * 8 + col];
        }

        idct_1d(&col_in, &mut col_out, &cos_table_neon);

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
fn idct_1d(input: &[f32], output: &mut [f32], cos_table: &CosTableNeon) {
    unsafe {
        let in_lo = vld1q_f32(input.as_ptr());
        let in_hi = vld1q_f32(input.as_ptr().add(4));

        for x in 0..8 {
            // Multiply-accumulate
            let cos_lo = cos_table.lo[x];
            let cos_hi = cos_table.hi[x];

            let mut sum = vmulq_f32(in_lo, cos_lo);
            sum = vfmaq_f32(sum, in_hi, cos_hi);

            output[x] = vaddvq_f32(sum) * 0.5;
        }
    }
}
