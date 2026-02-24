// BT.601 full-range fixed-point coefficients (14-bit precision).
//
// Computed with MPFR for maximum accuracy.  Using i32 arithmetic
// so the compiler can auto-vectorize when appropriate.
const Y_COEFF: i32 = 16384; //  1.0      × 2¹⁴
const CR_R_COEFF: i32 = 22970; //  1.402    × 2¹⁴
const CB_B_COEFF: i32 = 29032; //  1.772    × 2¹⁴
const CR_G_COEFF: i32 = -11700; // -0.714136 × 2¹⁴
const CB_G_COEFF: i32 = -5638; // -0.344136 × 2¹⁴
const PRECISION: i32 = 14;
const ROUND: i32 = (1 << (PRECISION - 1)) - 1;

/// Batch YCbCr→RGB conversion using fixed-point BT.601 full-range coefficients.
///
/// Converts `y.len()` pixels.  `cb` and `cr` must have at least as many
/// elements as `y`, and `rgb` must have at least `y.len() * 3` bytes.
pub fn ycbcr_to_rgb(y: &[i32], cb: &[i32], cr: &[i32], rgb: &mut [u8]) {
    let n = y.len();
    for i in 0..n {
        let y0 = y[i] * Y_COEFF + ROUND;
        let cb0 = cb[i] - 128;
        let cr0 = cr[i] - 128;

        let r = (y0 + cr0 * CR_R_COEFF) >> PRECISION;
        let g = (y0 + cr0 * CR_G_COEFF + cb0 * CB_G_COEFF) >> PRECISION;
        let b = (y0 + cb0 * CB_B_COEFF) >> PRECISION;

        rgb[i * 3] = r.clamp(0, 255) as u8;
        rgb[i * 3 + 1] = g.clamp(0, 255) as u8;
        rgb[i * 3 + 2] = b.clamp(0, 255) as u8;
    }
}
