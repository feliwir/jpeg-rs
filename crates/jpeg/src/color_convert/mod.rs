mod scalar;
mod simd;

pub fn select_ycbcr_to_rgb_converter()
-> fn(&[i16; 16], &[i16; 16], &[i16; 16], &mut [u8], &mut usize) {
    return scalar::ycbcr_to_rgb_scalar;
}
