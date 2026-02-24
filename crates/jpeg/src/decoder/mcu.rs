//! MCU (Minimum Coded Unit) decoding for baseline JPEG.
//!
//! This module implements the core decoding pipeline:
//!
//! 1. **Huffman decode** – read DC and AC coefficients from the compressed bitstream
//! 2. **Dequantize**    – multiply coefficients by the quantization table
//! 3. **Un-zigzag**     – reorder coefficients from zigzag scan to row-major order
//! 4. **Inverse DCT**   – transform 8×8 frequency-domain block to spatial pixels
//! 5. **Color convert** – convert YCbCr to RGB and write to the output buffer

use crate::{
    color_convert, constants::UN_ZIGZAG, error::DecodeError, huffman::HuffmanTable, io::BitReader,
};

use super::JpegDecoder;

// ── Decode entry point ──────────────────────────────────────────────────────

impl<R: std::io::Read> JpegDecoder<R> {
    /// Validate that every component references tables that actually exist.
    fn check_tables(&self) -> Result<(), DecodeError> {
        for comp in &self.components {
            if self.quantization_tables[comp.quantization_table_id].is_none() {
                return Err(DecodeError::Format(format!(
                    "Component {} references missing quantization table {}",
                    comp.id, comp.quantization_table_id
                )));
            }
            if self.dc_huffman_tables[comp.dc_huffman_table_id].is_none() {
                return Err(DecodeError::Format(format!(
                    "Component {} references missing DC Huffman table {}",
                    comp.id, comp.dc_huffman_table_id
                )));
            }
            if self.ac_huffman_tables[comp.ac_huffman_table_id].is_none() {
                return Err(DecodeError::Format(format!(
                    "Component {} references missing AC Huffman table {}",
                    comp.id, comp.ac_huffman_table_id
                )));
            }
        }
        Ok(())
    }

    /// Compute MCU grid dimensions from the component sampling factors.
    fn setup_mcu_params(&mut self) {
        // Find the maximum sampling factors across all components
        for comp in &self.components {
            self.h_max = self.h_max.max(comp.horizontal_sampling_factor);
            self.v_max = self.v_max.max(comp.vertical_sampling_factor);
        }

        // MCU size in pixels
        self.mcu_width = self.h_max * 8;
        self.mcu_height = self.v_max * 8;

        // Number of MCUs in each direction (round up for partial MCUs at edges)
        self.mcu_x = self.info.width.div_ceil(self.mcu_width);
        self.mcu_y = self.info.height.div_ceil(self.mcu_height);
    }

    /// Decode all MCUs and write pixels to the output buffer.
    ///
    /// Output format:
    /// - **Grayscale** (1 component): 1 byte per pixel
    /// - **YCbCr** (3 components): 3 bytes per pixel (converted to RGB)
    pub(crate) fn decode_mcu_ycbcr(&mut self, output: &mut [u8]) -> Result<(), DecodeError> {
        self.check_tables()?;
        self.setup_mcu_params();

        if self.info.components != 1 && self.info.components != 3 {
            return Err(DecodeError::Unsupported(format!(
                "Only grayscale (1) and YCbCr (3) components are supported, got {}",
                self.info.components
            )));
        }

        // Copy structural parameters so we can borrow the reader separately
        let mcu_x = self.mcu_x;
        let mcu_y = self.mcu_y;
        let mcu_w = self.mcu_width;
        let mcu_h = self.mcu_height;
        let h_max = self.h_max;
        let v_max = self.v_max;
        let img_w = self.info.width;
        let img_h = self.info.height;
        let num_components = self.info.components;
        let bytes_per_component = if self.info.precision > 8 { 2 } else { 1 };
        let bytes_per_pixel = bytes_per_component * num_components;

        // Snapshot per-component info (sampling factors, table IDs)
        let comp_params: Vec<ComponentParams> = self
            .components
            .iter()
            .map(|c| ComponentParams {
                h_samples: c.horizontal_sampling_factor,
                v_samples: c.vertical_sampling_factor,
                qt_id: c.quantization_table_id,
                dc_ht_id: c.dc_huffman_table_id,
                ac_ht_id: c.ac_huffman_table_id,
            })
            .collect();

        // DC coefficients are differentially encoded; track previous value per component
        let mut prev_dc = vec![0i32; num_components];

        // Restart interval (0 = no restarts)
        let restart_interval = self.restart_interval;
        let mut mcu_count: usize = 0;

        // Wrap the byte reader in a bit reader for entropy-coded data
        let mut reader = BitReader::new(&mut self.reader);

        // Temporary storage for the decoded 8×8 blocks within one MCU
        let mut mcu_blocks: Vec<Vec<[i32; 64]>> = comp_params
            .iter()
            .map(|p| vec![[0i32; 64]; p.h_samples * p.v_samples])
            .collect();

        // Sampling factors for color conversion
        let h_samples: Vec<usize> = comp_params.iter().map(|p| p.h_samples).collect();
        let v_samples: Vec<usize> = comp_params.iter().map(|p| p.v_samples).collect();

        // ── Main MCU loop ───────────────────────────────────────────────
        for mcu_row in 0..mcu_y {
            for mcu_col in 0..mcu_x {
                // Handle restart markers between entropy-coded segments.
                // RST markers reset DC prediction and appear every
                // `restart_interval` MCUs (when the interval is non-zero).
                if restart_interval > 0 && mcu_count > 0 && mcu_count % restart_interval == 0 {
                    let marker = reader.read_restart_marker()?;
                    if !matches!(marker, 0xD0..=0xD7) {
                        return Err(DecodeError::Format(format!(
                            "Expected RST marker, found 0xFF{marker:02X}"
                        )));
                    }
                    prev_dc.fill(0);
                }
                // Step 1: Decode all 8×8 blocks for every component in this MCU.
                //
                // In an interleaved scan the blocks appear in component order;
                // within each component the blocks are arranged in raster order
                // of the sampling grid (left→right, top→bottom).
                for (ci, params) in comp_params.iter().enumerate() {
                    for v in 0..params.v_samples {
                        for h in 0..params.h_samples {
                            let block = &mut mcu_blocks[ci][v * params.h_samples + h];
                            *block = [0i32; 64];

                            decode_block(
                                &mut reader,
                                self.idct_fn,
                                self.dc_huffman_tables[params.dc_ht_id].as_ref().unwrap(),
                                self.ac_huffman_tables[params.ac_ht_id].as_ref().unwrap(),
                                self.quantization_tables[params.qt_id].as_ref().unwrap(),
                                &mut prev_dc[ci],
                                block,
                            )?;
                        }
                    }
                }

                // Step 2: Convert blocks → output pixels.
                color_convert::write_mcu_pixels(
                    self.ycbcr_to_rgb_fn,
                    &mcu_blocks,
                    &h_samples,
                    &v_samples,
                    h_max,
                    v_max,
                    mcu_w,
                    mcu_h,
                    mcu_col,
                    mcu_row,
                    img_w,
                    img_h,
                    num_components,
                    bytes_per_pixel,
                    output,
                );

                mcu_count += 1;
            }
        }

        Ok(())
    }
}

// ── Helper types ────────────────────────────────────────────────────────────

/// Per-component parameters snapshotted before the decode loop so we don't
/// need to borrow `self.components` while the reader is also borrowed.
struct ComponentParams {
    h_samples: usize,
    v_samples: usize,
    qt_id: usize,
    dc_ht_id: usize,
    ac_ht_id: usize,
}

// ── Block-level decoding ────────────────────────────────────────────────────

/// Decode one 8×8 block: Huffman decode → dequantize → un-zigzag → IDCT.
///
/// After this function returns, `block` contains 64 pixel values in row-major
/// order, each clamped to `[0, 255]`.
fn decode_block<R: std::io::Read>(
    reader: &mut BitReader<R>,
    idct_fn: unsafe fn(&mut [i32; 64]),
    dc_table: &HuffmanTable,
    ac_table: &HuffmanTable,
    quant_table: &[i32; 64],
    prev_dc: &mut i32,
    block: &mut [i32; 64],
) -> Result<(), DecodeError> {
    // ── DC coefficient ──────────────────────────────────────────────
    // DC values are differentially encoded: actual = previous + decoded_diff
    let dc_category = dc_table.decode(reader)?;
    let dc_diff = if dc_category > 0 {
        let bits = reader.read_bits(dc_category)?;
        receive_extend(bits, dc_category)
    } else {
        0
    };
    *prev_dc += dc_diff;

    // Store coefficients in zigzag order (un-zigzagged below)
    let mut coeffs = [0i32; 64];
    coeffs[0] = *prev_dc;

    // ── AC coefficients (zigzag positions 1–63) ─────────────────────
    let mut k = 1usize;
    while k < 64 {
        let rs = ac_table.decode(reader)?;
        let run = (rs >> 4) as usize; // number of preceding zero coefficients
        let size = rs & 0x0F; // bit-width of the coefficient value

        if size == 0 {
            if run == 0 {
                break; // EOB — remaining coefficients are all zero
            }
            // ZRL (run == 15): skip 16 zero positions
            k += 16;
            continue;
        }

        k += run;
        if k >= 64 {
            return Err(DecodeError::Format(
                "AC coefficient index out of range (>63)".to_string(),
            ));
        }

        let bits = reader.read_bits(size)?;
        coeffs[k] = receive_extend(bits, size);
        k += 1;
    }

    // ── Un-zigzag and dequantize ────────────────────────────────────
    // Coefficients arrive in zigzag scan order.  We place each one at its
    // natural row-major position and multiply by the corresponding
    // quantization table entry (the table was already un-zigzagged when
    // it was read from the DQT marker).
    for i in 0..64 {
        let natural = UN_ZIGZAG[i];
        block[natural] = coeffs[i] * quant_table[natural];
    }

    // ── Inverse DCT → pixel values ──────────────────────────────────
    unsafe { (idct_fn)(block) };

    Ok(())
}

/// Extend a raw bit pattern to a signed coefficient value (JPEG Figure F.12).
///
/// For a category of `size` bits, values in the lower half of the range are
/// negative.  For example with `size = 3`:
///   - bits `100`–`111` (4–7)  →  4–7  (positive)
///   - bits `000`–`011` (0–3)  → −7–−4 (negative)
fn receive_extend(bits: u16, size: u8) -> i32 {
    let vt = 1 << (size as i32 - 1);
    if (bits as i32) < vt {
        bits as i32 + (-1 << size as i32) + 1
    } else {
        bits as i32
    }
}
