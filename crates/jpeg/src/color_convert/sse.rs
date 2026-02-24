use std::arch::x86_64::*;

// BT.601 full-range fixed-point coefficients (14-bit precision).
const Y_COEFF: i32 = 16384;
const CR_R_COEFF: i32 = 22970;
const CB_B_COEFF: i32 = 29032;
const CR_G_COEFF: i32 = -11700;
const CB_G_COEFF: i32 = -5638;
const ROUND: i32 = (1 << 13) - 1;

/// Batch YCbCr→RGB conversion using SSE4.1 intrinsics.
///
/// Processes 4 pixels at a time with 128-bit integer SIMD.
/// Falls back to the scalar path for any remainder pixels.
#[target_feature(enable = "sse4.1")]
pub fn ycbcr_to_rgb(y: &[i32], cb: &[i32], cr: &[i32], rgb: &mut [u8]) {
    unsafe {
        let n = y.len();
        let chunks = n / 4;
        let remainder = n % 4;

        let y_cf = _mm_set1_epi32(Y_COEFF);
        let rnd = _mm_set1_epi32(ROUND);
        let cr_r = _mm_set1_epi32(CR_R_COEFF);
        let cb_b = _mm_set1_epi32(CB_B_COEFF);
        let cr_g = _mm_set1_epi32(CR_G_COEFF);
        let cb_g = _mm_set1_epi32(CB_G_COEFF);
        let offset = _mm_set1_epi32(128);
        let zero = _mm_setzero_si128();
        let max_val = _mm_set1_epi32(255);

        // Shuffle mask: planar [R0..R3, G0..G3, B0..B3, xxxx]
        //            → interleaved [R0,G0,B0, R1,G1,B1, R2,G2,B2, R3,G3,B3]
        let shuffle = _mm_setr_epi8(0, 4, 8, 1, 5, 9, 2, 6, 10, 3, 7, 11, -1, -1, -1, -1);

        for i in 0..chunks {
            let base = i * 4;

            let yv = _mm_loadu_si128(y.as_ptr().add(base) as *const __m128i);
            let cbv = _mm_sub_epi32(
                _mm_loadu_si128(cb.as_ptr().add(base) as *const __m128i),
                offset,
            );
            let crv = _mm_sub_epi32(
                _mm_loadu_si128(cr.as_ptr().add(base) as *const __m128i),
                offset,
            );

            // y0 = y × Y_COEFF + ROUND
            let y0 = _mm_add_epi32(_mm_mullo_epi32(yv, y_cf), rnd);

            // r = (y0 + cr × CR_R) >> 14
            let r = _mm_srai_epi32::<14>(_mm_add_epi32(y0, _mm_mullo_epi32(crv, cr_r)));

            // g = (y0 + cr × CR_G + cb × CB_G) >> 14
            let g = _mm_srai_epi32::<14>(_mm_add_epi32(
                _mm_add_epi32(y0, _mm_mullo_epi32(crv, cr_g)),
                _mm_mullo_epi32(cbv, cb_g),
            ));

            // b = (y0 + cb × CB_B) >> 14
            let b = _mm_srai_epi32::<14>(_mm_add_epi32(y0, _mm_mullo_epi32(cbv, cb_b)));

            // Clamp to [0, 255]
            let r = _mm_min_epi32(_mm_max_epi32(r, zero), max_val);
            let g = _mm_min_epi32(_mm_max_epi32(g, zero), max_val);
            let b = _mm_min_epi32(_mm_max_epi32(b, zero), max_val);

            // Pack i32 → i16 → u8, then shuffle to interleaved RGB
            let rg16 = _mm_packs_epi32(r, g);
            let bz16 = _mm_packs_epi32(b, zero);
            let rgbz = _mm_packus_epi16(rg16, bz16);
            let interleaved = _mm_shuffle_epi8(rgbz, shuffle);

            // Store 12 bytes of interleaved RGB
            let tmp: [u8; 16] = std::mem::transmute(interleaved);
            let out_base = base * 3;
            rgb[out_base..out_base + 12].copy_from_slice(&tmp[..12]);
        }

        // Scalar fallback for remaining pixels
        if remainder > 0 {
            let start = chunks * 4;
            super::scalar::ycbcr_to_rgb(
                &y[start..],
                &cb[start..],
                &cr[start..],
                &mut rgb[start * 3..],
            );
        }
    }
}
