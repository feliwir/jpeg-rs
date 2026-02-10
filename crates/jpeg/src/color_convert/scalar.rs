// Bt.601 Full Range inverse coefficients computed with 14 bits of precision with MPFR.
// This is important to keep them in i16.
// In most cases LLVM will detect what we're doing i16 widening to i32 math and will use
// appropriate optimizations.
const Y_CF: i16 = 16384;
const CR_CF: i16 = 22970;
const CB_CF: i16 = 29032;
const C_G_CR_COEF_1: i16 = -11700;
const C_G_CB_COEF_2: i16 = -5638;
const YUV_PREC: i16 = 14;
// Rounding const for YUV -> RGB conversion: floating equivalent 0.499(9).
const YUV_RND: i16 = (1 << (YUV_PREC - 1)) - 1;

/// Convert YCbCr to RGB
///
/// Converts to a slice of 16 RGB pixels (48 bytes) starting at `output[pos]`.
/// The Y, Cb, and Cr inputs are 16 i16 values each, representing the dequantized DCT coefficients for one 8x8 block.
/// The output is a slice of 48 u8 values, where each group of 3 bytes corresponds to the R, G, and B values of a pixel.
pub fn ycbcr_to_rgb_scalar(
    y: &[i16; 16],
    cb: &[i16; 16],
    cr: &[i16; 16],
    output: &mut [u8],
    pos: &mut usize,
) {
    let (_, output_position) = output.split_at_mut(*pos);

    // Convert into a slice with 48 elements
    let opt: &mut [u8; 48] = output_position
        .get_mut(0..48)
        .expect("Slice to small cannot write")
        .try_into()
        .unwrap();

    for ((&y, (cb, cr)), out) in y
        .iter()
        .zip(cb.iter().zip(cr.iter()))
        .zip(opt.chunks_exact_mut(3))
    {
        let cr = cr - 128;
        let cb = cb - 128;

        let y0 = i32::from(y) * i32::from(Y_CF) + i32::from(YUV_RND);

        let r = (y0 + i32::from(cr) * i32::from(CR_CF)) >> YUV_PREC;
        let g = (y0
            + i32::from(cr) * i32::from(C_G_CR_COEF_1)
            + i32::from(cb) * i32::from(C_G_CB_COEF_2))
            >> YUV_PREC;
        let b = (y0 + i32::from(cb) * i32::from(CB_CF)) >> YUV_PREC;

        out[0] = r.clamp(0, 255) as u8;
        out[1] = g.clamp(0, 255) as u8;
        out[2] = b.clamp(0, 255) as u8;
    }

    // Increment pos
    *pos += 48;
}
