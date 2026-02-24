use super::{CB_B_COEFF, CB_G_COEFF, CR_G_COEFF, CR_R_COEFF, PRECISION, ROUND, Y_COEFF};

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
