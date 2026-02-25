//! Baseline encoder implementation
//!
//! Encoding pipeline (inverse of the decoder):
//!
//! 1. **Level shift**   – subtract 128 from each pixel value
//! 2. **Forward DCT**   – transform 8×8 spatial block to frequency domain
//! 3. **Quantize**      – divide coefficients by quantization table entries
//! 4. **Zigzag**        – reorder coefficients from row-major to zigzag scan order
//! 5. **Huffman encode** – write DC and AC coefficients to the compressed bitstream

use crate::component::Component;
use crate::constants::ZIGZAG;
use crate::dct;
use crate::error::EncodeError;
use crate::huffman_encode::{
    AC_CHROM_LENGTHS, AC_CHROM_VALUES, AC_LUM_LENGTHS, AC_LUM_VALUES, DC_CHROM_LENGTHS,
    DC_CHROM_VALUES, DC_LUM_LENGTHS, DC_LUM_VALUES, HuffmanEncodeTable, encode_coefficient,
};
use crate::io::BitWriter;
use crate::marker::{
    Marker, write_app0, write_dht, write_dqt, write_marker, write_sof0, write_sos,
};
use jpeg_common::color_space::ColorSpace;
use jpeg_common::options::EncoderOptions;
use std::io::Write;

// ── Standard quantization tables (ITU-T T.81, Annex K, Table K.1 & K.2) ────

#[rustfmt::skip]
const BASE_LUMA_QT: [u8; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61,
    12, 12, 14, 19, 26, 58, 60, 55,
    14, 13, 16, 24, 40, 57, 69, 56,
    14, 17, 22, 29, 51, 87, 80, 62,
    18, 22, 37, 56, 68,109,103, 77,
    24, 35, 55, 64, 81,104,113, 92,
    49, 64, 78, 87,103,121,120,101,
    72, 92, 95, 98,112,100,103, 99,
];

#[rustfmt::skip]
const BASE_CHROMA_QT: [u8; 64] = [
    17, 18, 24, 47, 99, 99, 99, 99,
    18, 21, 26, 66, 99, 99, 99, 99,
    24, 26, 56, 99, 99, 99, 99, 99,
    47, 66, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
];

/// Compute a quality-scaled quantization table.
///
/// Uses the IJG quality scaling formula (same as `write_dqt`).
fn scaled_quant_table(base: &[u8; 64], quality: u8) -> [i32; 64] {
    let q = if quality < 50 {
        (5000 / quality as i32) as u8
    } else {
        (200 - 2 * quality as i32) as u8
    }
    .max(1);

    let mut table = [0i32; 64];
    for i in 0..64 {
        table[i] = ((base[i] as u32 * q as u32) / 100).max(1).min(255) as i32;
    }
    table
}

/// Convert the user-facing chroma subsampling notation (J:a style) to JPEG
/// sampling factors for the Y component.  Cb and Cr always use 1×1.
///
/// | Input     | Standard | Y factors |
/// |-----------|----------|-----------|
/// | `(4, 4)`  | 4:4:4    | 1 × 1     |
/// | `(4, 2)`  | 4:2:0    | 2 × 2     |
/// | `(4, 1)`  | 4:2:2    | 2 × 1     |
/// | `(h, v)` with h,v ≤ 4 | direct | h × v |
fn sampling_factors(subsampling: (u8, u8)) -> (u8, u8) {
    match subsampling {
        (4, 4) => (1, 1), // 4:4:4 – no chroma subsampling
        (4, 2) => (2, 2), // 4:2:0 – half in both dimensions
        (4, 1) => (2, 1), // 4:2:2 – half horizontally only
        (h, v) => (h, v), // treat as direct sampling factors
    }
}

/// Build the component list for the given colorspace and subsampling.
fn build_components(colorspace: ColorSpace, h_samp: u8, v_samp: u8) -> Vec<Component> {
    match colorspace {
        ColorSpace::Grayscale => vec![Component::new(1, 1, 1, 0, 0, 0)],
        // YCbCr: Y gets the requested subsampling, Cb/Cr always 1×1.
        _ => vec![
            Component::new(1, h_samp as usize, v_samp as usize, 0, 0, 0),
            Component::new(2, 1, 1, 1, 1, 1),
            Component::new(3, 1, 1, 1, 1, 1),
        ],
    }
}

/// Encode image data to baseline JPEG.
///
/// Input is expected to be raw samples in the colorspace specified by
/// `options.colorspace()` — Grayscale (1 byte/pixel) or YCbCr
/// (3 bytes/pixel).  Any RGB→YCbCr conversion must happen before calling
/// this function.
pub(crate) fn encode_baseline<W: Write>(
    writer: &mut W,
    options: &EncoderOptions,
    data: &[u8],
) -> Result<(), EncodeError> {
    let width = options.width();
    let height = options.height();
    let quality = options.quality();
    let colorspace = options.colorspace();
    let (h_samp, v_samp) = sampling_factors(options.chroma_subsampling());

    let num_components = colorspace.num_components();
    let expected_size = width * height * num_components;
    if data.len() < expected_size {
        return Err(EncodeError::InvalidDimensions(format!(
            "Input data too small: expected {expected_size}, got {}",
            data.len()
        )));
    }

    let components = build_components(colorspace, h_samp, v_samp);

    // Build quantization tables (row-major order, matching DCT output)
    let luma_qt = scaled_quant_table(&BASE_LUMA_QT, quality);
    let chroma_qt = scaled_quant_table(&BASE_CHROMA_QT, quality);

    // Build Huffman encoding tables
    let dc_lum_ht = HuffmanEncodeTable::new(&DC_LUM_LENGTHS, DC_LUM_VALUES);
    let ac_lum_ht = HuffmanEncodeTable::new(&AC_LUM_LENGTHS, AC_LUM_VALUES);
    let dc_chrom_ht = HuffmanEncodeTable::new(&DC_CHROM_LENGTHS, DC_CHROM_VALUES);
    let ac_chrom_ht = HuffmanEncodeTable::new(&AC_CHROM_LENGTHS, AC_CHROM_VALUES);

    // Select the forward DCT function
    let dct_fn = dct::select_dct_fn(options.forced_simd_backend());

    // ── Write JPEG headers ──────────────────────────────────────────

    write_marker(writer, Marker::SOI)?;
    write_app0(writer)?;
    write_dqt(writer, quality)?;
    write_sof0(writer, width as u16, height as u16, 8, &components)?;
    write_dht(writer, num_components)?;
    write_sos(writer, &components)?;

    // ── Encode entropy-coded image data ─────────────────────────────

    let h_max = components
        .iter()
        .map(|c| c.horizontal_sampling_factor)
        .max()
        .unwrap_or(1);
    let v_max = components
        .iter()
        .map(|c| c.vertical_sampling_factor)
        .max()
        .unwrap_or(1);

    let mcu_width = h_max * 8;
    let mcu_height = v_max * 8;
    let mcu_x = width.div_ceil(mcu_width);
    let mcu_y = height.div_ceil(mcu_height);

    let mut bit_writer = BitWriter::new(writer);
    let mut prev_dc = vec![0i32; num_components];

    for mcu_row in 0..mcu_y {
        for mcu_col in 0..mcu_x {
            // Process each component in the MCU
            for ci in 0..num_components {
                let h_samp_c = components[ci].horizontal_sampling_factor;
                let v_samp_c = components[ci].vertical_sampling_factor;
                let qt = if components[ci].quantization_table_id == 0 {
                    &luma_qt
                } else {
                    &chroma_qt
                };
                let dc_ht = if components[ci].dc_huffman_table_id == 0 {
                    &dc_lum_ht
                } else {
                    &dc_chrom_ht
                };
                let ac_ht = if components[ci].ac_huffman_table_id == 0 {
                    &ac_lum_ht
                } else {
                    &ac_chrom_ht
                };

                // Each component may have multiple blocks in one MCU
                for v in 0..v_samp_c {
                    for h in 0..h_samp_c {
                        let mut block = [0i32; 64];

                        // Fill the 8×8 block from the input image
                        fill_block(
                            data,
                            width,
                            height,
                            num_components,
                            ci,
                            mcu_col,
                            mcu_row,
                            h_max,
                            v_max,
                            h_samp_c,
                            v_samp_c,
                            h,
                            v,
                            &mut block,
                        );

                        // Level shift: subtract 128
                        for val in block.iter_mut() {
                            *val -= 128;
                        }

                        // Forward DCT
                        unsafe { (dct_fn)(&mut block) };

                        // Quantize and zigzag
                        let mut zigzag = [0i32; 64];
                        for i in 0..64 {
                            let q = qt[i];
                            // Round to nearest: (coeff + q/2) / q for positive,
                            // (coeff - q/2) / q for negative
                            let coeff = block[i];
                            zigzag[ZIGZAG[i]] = if coeff >= 0 {
                                (coeff + q / 2) / q
                            } else {
                                (coeff - q / 2) / q
                            };
                        }

                        // Huffman encode (DC + AC)
                        encode_block(&mut bit_writer, &zigzag, &mut prev_dc[ci], dc_ht, ac_ht)?;
                    }
                }
            }
        }
    }

    // Flush remaining bits
    bit_writer.flush()?;

    // Write EOI marker
    let writer = bit_writer.get_mut();
    write_marker(writer, Marker::EOI)?;

    Ok(())
}

/// Fill an 8×8 block from the source image for a given component, block
/// position within the MCU, and MCU position within the image.
///
/// Handles edge padding by repeating the last valid pixel.
fn fill_block(
    data: &[u8],
    img_w: usize,
    img_h: usize,
    num_components: usize,
    component_index: usize,
    mcu_col: usize,
    mcu_row: usize,
    h_max: usize,
    v_max: usize,
    h_samp: usize,
    v_samp: usize,
    block_h: usize,
    block_v: usize,
    block: &mut [i32; 64],
) {
    // Pixel coordinates of the top-left of this 8×8 block
    let block_x0 = mcu_col * h_max * 8 + block_h * 8;
    let block_y0 = mcu_row * v_max * 8 + block_v * 8;

    // For subsampled components, we need to map block coordinates to
    // the component's own coordinate space.
    // Component pixel coordinate = block_pixel * (img_comp_size / mcu_grid_size)
    // which simplifies to scaling by samp / max
    // Step size: for Y (h_samp == h_max) step is 1; for subsampled
    // chroma the block must cover the full MCU area so step > 1.
    let step_x = h_max / h_samp;
    let step_y = v_max / v_samp;

    for row in 0..8 {
        for col in 0..8 {
            let src_x = block_x0 + col * step_x;
            let src_y = block_y0 + row * step_y;

            // Clamp to image bounds (repeat edge pixel)
            let sx = src_x.min(img_w - 1);
            let sy = src_y.min(img_h - 1);

            let pixel_idx = (sy * img_w + sx) * num_components + component_index;
            block[row * 8 + col] = data[pixel_idx] as i32;
        }
    }
}

/// Encode one 8×8 block: write DC coefficient followed by AC coefficients.
fn encode_block<W: Write>(
    writer: &mut BitWriter<W>,
    zigzag: &[i32; 64],
    prev_dc: &mut i32,
    dc_ht: &HuffmanEncodeTable,
    ac_ht: &HuffmanEncodeTable,
) -> Result<(), EncodeError> {
    // ── DC coefficient (differential encoding) ──────────────────────
    let dc_diff = zigzag[0] - *prev_dc;
    *prev_dc = zigzag[0];

    let (dc_category, dc_bits) = encode_coefficient(dc_diff);
    dc_ht.encode(writer, dc_category)?;
    if dc_category > 0 {
        writer.write_bits(dc_bits as u32, dc_category)?;
    }

    // ── AC coefficients (zigzag positions 1–63) ─────────────────────
    let mut zero_run = 0u8;

    for k in 1..64 {
        if zigzag[k] == 0 {
            zero_run += 1;
            continue;
        }

        // Emit ZRL (0xF0) for every 16 consecutive zeros
        while zero_run >= 16 {
            ac_ht.encode(writer, 0xF0)?; // ZRL
            zero_run -= 16;
        }

        let (category, bits) = encode_coefficient(zigzag[k]);
        let rs = (zero_run << 4) | category;
        ac_ht.encode(writer, rs)?;
        writer.write_bits(bits as u32, category)?;
        zero_run = 0;
    }

    // If we reach the end with trailing zeros, emit EOB
    if zero_run > 0 {
        ac_ht.encode(writer, 0x00)?; // EOB
    }

    Ok(())
}
