use super::{
    CB_B_COEFF, CB_G_COEFF, CR_G_COEFF, CR_R_COEFF, FWD_CB_B, FWD_CB_G, FWD_CB_R, FWD_CR_B,
    FWD_CR_G, FWD_CR_R, FWD_Y_B, FWD_Y_G, FWD_Y_R, PRECISION, ROUND, Y_COEFF,
};

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

/// Batch RGB→YCbCr conversion using fixed-point BT.601 full-range coefficients.
///
/// Converts interleaved RGB pixels to interleaved YCbCr in-place style.
/// `rgb` must have `n * 3` bytes; `ycbcr` must have at least `n * 3` bytes.
pub fn rgb_to_ycbcr(rgb: &[u8], ycbcr: &mut [u8]) {
    let n = rgb.len() / 3;
    for i in 0..n {
        let r = rgb[i * 3] as i32;
        let g = rgb[i * 3 + 1] as i32;
        let b = rgb[i * 3 + 2] as i32;

        let y = (FWD_Y_R * r + FWD_Y_G * g + FWD_Y_B * b + ROUND) >> PRECISION;
        let cb = (FWD_CB_R * r + FWD_CB_G * g + FWD_CB_B * b + 128 * Y_COEFF + ROUND) >> PRECISION;
        let cr = (FWD_CR_R * r + FWD_CR_G * g + FWD_CR_B * b + 128 * Y_COEFF + ROUND) >> PRECISION;

        ycbcr[i * 3] = y.clamp(0, 255) as u8;
        ycbcr[i * 3 + 1] = cb.clamp(0, 255) as u8;
        ycbcr[i * 3 + 2] = cr.clamp(0, 255) as u8;
    }
}
