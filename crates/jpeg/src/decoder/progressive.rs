//! Progressive JPEG decoding.
//!
//! Progressive JPEG encodes an image in multiple scans.  Each scan refines
//! a subset of DCT coefficients for one or more components.  The decoder
//! must accumulate all coefficients across scans before performing the
//! final dequantization, IDCT, and color conversion.
//!
//! ## Pull-based API
//!
//! The public API lets callers receive intermediate results after every scan:
//!
//! ```ignore
//! decoder.decode_headers()?;
//! let mut state = decoder.start_progressive()?;
//! let mut pixels = vec![0u8; decoder.required_buffer_size().unwrap()];
//! while decoder.decode_next_scan(&mut state)? {
//!     decoder.reconstruct(&state, &mut pixels);
//!     // `pixels` now contains the best image available so far
//! }
//! ```
//!
//! Scan types:
//!  - **DC first** (Ss=0, Se=0, Ah=0): initial DC coefficient, shifted by Al
//!  - **DC refining** (Ss=0, Se=0, Ah≠0): one correction bit per DC coefficient
//!  - **AC first** (Ss>0, Ah=0): initial AC coefficients in band [Ss..Se]
//!  - **AC refining** (Ss>0, Ah≠0): refine existing + insert new AC coefficients

use super::{JpegDecoder, alloc_mcu_blocks, baseline::receive_extend};
use crate::{
    color_convert::{self, ImageLayout, McuGeometry, SamplingParams, write_mcu_pixels},
    constants::UN_ZIGZAG,
    error::DecodeError,
    huffman::HuffmanTable,
    io::BitReader,
    marker::{self, Marker},
};

// ── Public state ────────────────────────────────────────────────────────────

/// Accumulated coefficient state for progressive JPEG decoding.
///
/// Holds per-component DCT coefficients (in zigzag order) that are refined
/// scan by scan.  Pass to [`JpegDecoder::reconstruct`] at any time to obtain
/// the best pixel data available so far.
pub struct ProgressiveState {
    /// Per-component coefficient buffers.  Each inner Vec has one `[i32; 64]`
    /// per 8×8 block, stored in raster order within the component's block grid.
    pub(crate) coeffs: Vec<Vec<[i32; 64]>>,
    /// Number of 8×8 blocks per row for each component.
    pub(crate) blocks_per_row: Vec<usize>,
    /// Per-component horizontal sampling factors (snapshot).
    pub(crate) h_samples: Vec<usize>,
    /// Per-component vertical sampling factors (snapshot).
    pub(crate) v_samples: Vec<usize>,
    /// Number of scans decoded so far.
    scan_count: usize,
    /// Whether the first scan has been decoded (the SOS parsed in
    /// `decode_headers` must be consumed before reading further markers).
    first_scan_pending: bool,
}

impl ProgressiveState {
    /// Number of scans decoded so far.
    pub fn scan_count(&self) -> usize {
        self.scan_count
    }
}

// ── Public entry points ─────────────────────────────────────────────────────

impl<R: std::io::BufRead> JpegDecoder<R> {
    /// Prepare for progressive decoding.
    ///
    /// Call this after [`decode_headers`](Self::decode_headers).
    /// Allocates the coefficient buffers and returns a [`ProgressiveState`]
    /// that must be passed to [`decode_next_scan`](Self::decode_next_scan)
    /// and [`reconstruct`](Self::reconstruct).
    pub fn start_progressive(&mut self) -> Result<ProgressiveState, DecodeError> {
        if !self.is_progressive {
            return Err(DecodeError::FormatStatic(
                "start_progressive called on a non-progressive JPEG",
            ));
        }

        self.setup_mcu_params();

        let num_components = self.components.len();
        if num_components != 1 && num_components != 3 {
            return Err(DecodeError::Unsupported(format!(
                "Only grayscale (1) and YCbCr (3) components are supported, got {}",
                num_components
            )));
        }

        let h_samples: Vec<usize> = self
            .components
            .iter()
            .map(|c| c.horizontal_sampling_factor)
            .collect();
        let v_samples: Vec<usize> = self
            .components
            .iter()
            .map(|c| c.vertical_sampling_factor)
            .collect();

        let mcu_x = self.mcu_x;
        let mcu_y = self.mcu_y;
        let blocks_per_row: Vec<usize> = h_samples.iter().map(|h| mcu_x * h).collect();

        let coeffs: Vec<Vec<[i32; 64]>> = (0..num_components)
            .map(|ci| {
                let rows = mcu_y * v_samples[ci];
                vec![[0i32; 64]; blocks_per_row[ci] * rows]
            })
            .collect();

        Ok(ProgressiveState {
            coeffs,
            blocks_per_row,
            h_samples,
            v_samples,
            scan_count: 0,
            first_scan_pending: true,
        })
    }

    /// Decode the next scan in the progressive JPEG stream.
    ///
    /// Returns `true` if a scan was decoded, `false` when the end of image
    /// is reached.  After each `true` return you can call
    /// [`reconstruct`](Self::reconstruct) to obtain intermediate pixels.
    pub fn decode_next_scan(&mut self, state: &mut ProgressiveState) -> Result<bool, DecodeError> {
        if state.first_scan_pending {
            // The first SOS was already parsed by decode_headers.
            state.first_scan_pending = false;
            self.decode_scan(
                &mut state.coeffs,
                &state.blocks_per_row,
                &state.h_samples,
                &state.v_samples,
            )?;
            state.scan_count += 1;
            return Ok(true);
        }

        // Read markers until we find the next SOS or hit EOI.
        loop {
            let marker = self.read_marker()?;
            match marker {
                Marker::SOS => {
                    marker::read_start_of_scan(self)?;
                    self.decode_scan(
                        &mut state.coeffs,
                        &state.blocks_per_row,
                        &state.h_samples,
                        &state.v_samples,
                    )?;
                    state.scan_count += 1;
                    return Ok(true);
                }
                Marker::DHT => {
                    marker::read_huffman_tables(self)?;
                }
                Marker::DRI => {
                    let length = self.read_length()?;
                    let data_len = length - 2;
                    if data_len != 2 {
                        return Err(DecodeError::Format(format!(
                            "Invalid DRI segment: expected 2 data bytes, got {data_len}"
                        )));
                    }
                    let mut buf = [0u8; 2];
                    self.reader.read_exact(&mut buf)?;
                    self.restart_interval = u16::from_be_bytes(buf) as usize;
                }
                Marker::EOI => return Ok(false),
                _ => {
                    let length = self.read_length()?;
                    self.skip_segment(length as u16)?;
                }
            }
        }
    }

    /// Reconstruct pixel data from the current coefficient state.
    ///
    /// Performs dequantization, inverse DCT, and color conversion on the
    /// accumulated coefficients.  Safe to call after any scan — earlier
    /// scans produce a coarser (but valid) preview of the final image.
    ///
    /// `output` must be at least [`required_buffer_size`](Self::required_buffer_size) bytes.
    pub fn reconstruct(
        &self,
        state: &ProgressiveState,
        output: &mut [u8],
    ) -> Result<(), DecodeError> {
        self.reconstruct_inner(
            output,
            &state.coeffs,
            &state.blocks_per_row,
            &state.h_samples,
            &state.v_samples,
        )
    }

    /// Convenience: decode all progressive scans and produce the final image.
    ///
    /// Called from `decode_into` when `is_progressive` is true.
    /// At this point the first SOS header has already been parsed.
    pub(crate) fn decode_progressive(&mut self, output: &mut [u8]) -> Result<(), DecodeError> {
        let mut state = self.start_progressive()?;
        while self.decode_next_scan(&mut state)? {}
        self.reconstruct(&state, output)?;
        Ok(())
    }
}

// ── Scan dispatch ───────────────────────────────────────────────────────────

impl<R: std::io::BufRead> JpegDecoder<R> {
    /// Decode one scan's entropy-coded data, updating the coefficient buffer.
    fn decode_scan(
        &mut self,
        coeffs: &mut [Vec<[i32; 64]>],
        blocks_per_row: &[usize],
        h_samples: &[usize],
        v_samples: &[usize],
    ) -> Result<(), DecodeError> {
        let ss = self.scan_ss;
        let se = self.scan_se;
        let ah = self.scan_ah;
        let al = self.scan_al;
        let scan_comps = self.scan_component_indices.clone();
        let mcu_x = self.mcu_x;
        let mcu_y = self.mcu_y;
        let restart_interval = self.restart_interval;

        log::trace!(
            "Processing scan: components={:?}, Ss={}, Se={}, Ah={}, Al={}",
            scan_comps,
            ss,
            se,
            ah,
            al
        );

        let is_dc = ss == 0 && se == 0;
        let is_first = ah == 0;
        let is_interleaved = scan_comps.len() > 1;

        // Snapshot the table IDs for the scan's components
        let dc_ht_ids: Vec<usize> = scan_comps
            .iter()
            .map(|&ci| self.components[ci].dc_huffman_table_id)
            .collect();
        let ac_ht_ids: Vec<usize> = scan_comps
            .iter()
            .map(|&ci| self.components[ci].ac_huffman_table_id)
            .collect();

        let mut prev_dc = vec![0i32; scan_comps.len()];
        let mut eob_run: u32 = 0;
        let mut mcu_count: usize = 0;

        let mut reader = BitReader::new(&mut self.reader);

        if is_dc && is_interleaved {
            // ── Interleaved DC scan ─────────────────────────────────
            for mcu_row in 0..mcu_y {
                for mcu_col in 0..mcu_x {
                    if restart_interval > 0 && mcu_count > 0 && mcu_count % restart_interval == 0 {
                        reader.read_restart_marker()?;
                        prev_dc.fill(0);
                    }

                    for (si, &ci) in scan_comps.iter().enumerate() {
                        for v in 0..v_samples[ci] {
                            for h in 0..h_samples[ci] {
                                let bx = mcu_col * h_samples[ci] + h;
                                let by = mcu_row * v_samples[ci] + v;
                                let idx = by * blocks_per_row[ci] + bx;
                                let block = &mut coeffs[ci][idx];

                                if is_first {
                                    decode_dc_first(
                                        &mut reader,
                                        self.dc_huffman_tables[dc_ht_ids[si]].as_ref().unwrap(),
                                        &mut prev_dc[si],
                                        al,
                                        block,
                                    )?;
                                } else {
                                    decode_dc_refine(&mut reader, al, block)?;
                                }
                            }
                        }
                    }

                    mcu_count += 1;
                }
            }
        } else if is_dc {
            // ── Non-interleaved DC scan (single component) ──────────
            let ci = scan_comps[0];
            let bpr = blocks_per_row[ci];
            let total_blocks = bpr * (mcu_y * v_samples[ci]);

            for blk_idx in 0..total_blocks {
                if restart_interval > 0 && blk_idx > 0 && blk_idx % restart_interval == 0 {
                    reader.read_restart_marker()?;
                    prev_dc[0] = 0;
                }

                let block = &mut coeffs[ci][blk_idx];
                if is_first {
                    decode_dc_first(
                        &mut reader,
                        self.dc_huffman_tables[dc_ht_ids[0]].as_ref().unwrap(),
                        &mut prev_dc[0],
                        al,
                        block,
                    )?;
                } else {
                    decode_dc_refine(&mut reader, al, block)?;
                }
            }
        } else {
            // ── Non-interleaved AC scan (single component) ──────────
            let ci = scan_comps[0];
            let bpr = blocks_per_row[ci];
            let total_blocks = bpr * (mcu_y * v_samples[ci]);

            for blk_idx in 0..total_blocks {
                if restart_interval > 0 && blk_idx > 0 && blk_idx % restart_interval == 0 {
                    reader.read_restart_marker()?;
                    eob_run = 0;
                }

                let block = &mut coeffs[ci][blk_idx];
                if is_first {
                    decode_ac_first(
                        &mut reader,
                        self.ac_huffman_tables[ac_ht_ids[0]].as_ref().unwrap(),
                        ss,
                        se,
                        al,
                        &mut eob_run,
                        block,
                    )?;
                } else {
                    decode_ac_refine(
                        &mut reader,
                        self.ac_huffman_tables[ac_ht_ids[0]].as_ref().unwrap(),
                        ss,
                        se,
                        al,
                        &mut eob_run,
                        block,
                    )?;
                }
            }
        }

        reader.discard_buffered_bits();
        Ok(())
    }
}

// ── DC scan decoders ────────────────────────────────────────────────────────

/// Decode a DC coefficient for the first time (Ah == 0).
///
/// Huffman-decodes the DC difference, applies differential prediction,
/// and stores the coefficient shifted left by `al`.
fn decode_dc_first<R: std::io::Read>(
    reader: &mut BitReader<R>,
    dc_table: &HuffmanTable,
    prev_dc: &mut i32,
    al: u8,
    block: &mut [i32; 64],
) -> Result<(), DecodeError> {
    let category = dc_table.decode(reader)?;
    let diff = if category > 0 {
        let bits = reader.read_bits(category)?;
        receive_extend(bits, category)
    } else {
        0
    };
    *prev_dc += diff;
    block[0] = *prev_dc << al;
    Ok(())
}

/// Refine a DC coefficient (Ah != 0).
///
/// Reads one correction bit and adds it to the existing coefficient.
fn decode_dc_refine<R: std::io::Read>(
    reader: &mut BitReader<R>,
    al: u8,
    block: &mut [i32; 64],
) -> Result<(), DecodeError> {
    let bit = reader.read_bit()?;
    block[0] |= (bit as i32) << al;
    Ok(())
}

// ── AC scan decoders ────────────────────────────────────────────────────────

/// Decode AC coefficients for the first time in band [ss..se] (Ah == 0).
///
/// Handles EOBn runs (multiple blocks with zero coefficients in this band)
/// and ZRL (skip 16 zeros). Coefficients are stored shifted left by `al`.
fn decode_ac_first<R: std::io::Read>(
    reader: &mut BitReader<R>,
    ac_table: &HuffmanTable,
    ss: u8,
    se: u8,
    al: u8,
    eob_run: &mut u32,
    block: &mut [i32; 64],
) -> Result<(), DecodeError> {
    // If we're inside an EOB run, this block has all zeros in the band
    if *eob_run > 0 {
        *eob_run -= 1;
        return Ok(());
    }

    let mut k = ss as usize;
    while k <= se as usize {
        let rs = ac_table.decode(reader)?;
        let run = (rs >> 4) as usize;
        let size = rs & 0x0F;

        if size == 0 {
            if run == 15 {
                // ZRL: skip 16 zero positions
                k += 16;
                continue;
            } else {
                // EOBn: end of band for this and the next (2^run + extra - 1) blocks
                *eob_run = (1u32 << run) - 1;
                if run > 0 {
                    let extra = reader.read_bits(run as u8)? as u32;
                    *eob_run += extra;
                }
                return Ok(());
            }
        }

        k += run;
        if k > se as usize {
            return Err(DecodeError::Format(
                "AC coefficient index out of band range in progressive scan".to_string(),
            ));
        }

        let bits = reader.read_bits(size)?;
        let coeff = receive_extend(bits, size);
        block[k] = coeff << al;
        k += 1;
    }

    Ok(())
}

/// Refine AC coefficients in band [ss..se] (Ah != 0).
///
/// This is the most complex progressive scan type.  It can both:
/// - Insert new non-zero coefficients (discovered via Huffman coding)
/// - Refine existing non-zero coefficients (one correction bit each)
///
/// When skipping `run` zero-valued positions, any already-non-zero positions
/// encountered along the way do NOT count toward the run — but each gets a
/// correction bit read and applied.
fn decode_ac_refine<R: std::io::Read>(
    reader: &mut BitReader<R>,
    ac_table: &HuffmanTable,
    ss: u8,
    se: u8,
    al: u8,
    eob_run: &mut u32,
    block: &mut [i32; 64],
) -> Result<(), DecodeError> {
    let p1 = 1i32 << al; // +1 at the current bit position
    let m1 = (-1i32) << al; // -1 at the current bit position

    let mut k = ss as usize;

    if *eob_run > 0 {
        // Inside an EOB run: just refine existing non-zero coefficients
        while k <= se as usize {
            if block[k] != 0 {
                apply_correction_bit(reader, al, &mut block[k])?;
            }
            k += 1;
        }
        *eob_run -= 1;
        return Ok(());
    }

    while k <= se as usize {
        let rs = ac_table.decode(reader)?;
        let run = (rs >> 4) as usize;
        let size = (rs & 0x0F) as usize;

        // Determine the new coefficient value (if any) before we start
        // scanning through positions, because the sign bit is read now.
        let new_value = if size == 0 {
            0
        } else if size == 1 {
            let bit = reader.read_bit()?;
            if bit == 1 { p1 } else { m1 }
        } else {
            return Err(DecodeError::Format(format!(
                "Invalid size {size} in AC refining scan (must be 0 or 1)"
            )));
        };

        if size == 0 && run != 15 {
            // EOBn
            *eob_run = (1u32 << run) - 1;
            if run > 0 {
                let extra = reader.read_bits(run as u8)? as u32;
                *eob_run += extra;
            }
            // Refine remaining non-zero coefficients in band
            while k <= se as usize {
                if block[k] != 0 {
                    apply_correction_bit(reader, al, &mut block[k])?;
                }
                k += 1;
            }
            return Ok(());
        }

        // Skip `run` zero-valued positions (refining non-zeros along the way)
        let mut zeros_remaining = run;
        while k <= se as usize {
            if block[k] != 0 {
                // Existing non-zero: apply correction bit
                apply_correction_bit(reader, al, &mut block[k])?;
                k += 1;
            } else if zeros_remaining > 0 {
                // This zero position counts toward the run
                zeros_remaining -= 1;
                k += 1;
            } else {
                // Found the target position for the new coefficient
                break;
            }
        }

        // Place the new non-zero coefficient (if size == 1)
        if new_value != 0 && k <= se as usize {
            block[k] = new_value;
        }
        k += 1;
    }

    Ok(())
}

/// Read one correction bit and apply it to an existing non-zero coefficient.
///
/// If the bit is 1: add `1 << al` in the direction of the coefficient's sign.
/// If the bit is 0: no change.
fn apply_correction_bit<R: std::io::Read>(
    reader: &mut BitReader<R>,
    al: u8,
    coeff: &mut i32,
) -> Result<(), DecodeError> {
    let bit = reader.read_bit()?;
    if bit != 0 {
        if *coeff > 0 {
            *coeff += 1 << al;
        } else {
            *coeff -= 1 << al;
        }
    }
    Ok(())
}

// ── Final reconstruction ────────────────────────────────────────────────────

impl<R: std::io::BufRead> JpegDecoder<R> {
    /// Perform final reconstruction: dequantize, IDCT, and color convert
    /// all accumulated progressive coefficients into the output buffer.
    fn reconstruct_inner(
        &self,
        output: &mut [u8],
        coeffs: &[Vec<[i32; 64]>],
        blocks_per_row: &[usize],
        h_samples: &[usize],
        v_samples: &[usize],
    ) -> Result<(), DecodeError> {
        let mcu_x = self.mcu_x;
        let mcu_y = self.mcu_y;
        let mcu_w = self.mcu_width;
        let mcu_h = self.mcu_height;
        let h_max = self.h_max;
        let v_max = self.v_max;
        let img_w = self.info.width;
        let img_h = self.info.height;
        let num_components = self.components.len();
        let output_format = self.output_format()?;
        let sampling = SamplingParams {
            h_samples,
            v_samples,
            h_max,
            v_max,
        };
        let image = ImageLayout {
            width: img_w,
            height: img_h,
        };

        // Temporary storage for the reconstructed 8×8 blocks within one MCU
        let mut mcu_blocks = alloc_mcu_blocks(h_samples, v_samples);

        for mcu_row in 0..mcu_y {
            for mcu_col in 0..mcu_x {
                // For each component, dequantize + IDCT its blocks
                for ci in 0..num_components {
                    let qt = self.quantization_tables[self.components[ci].quantization_table_id]
                        .as_ref()
                        .unwrap();

                    for v in 0..v_samples[ci] {
                        for h in 0..h_samples[ci] {
                            let bx = mcu_col * h_samples[ci] + h;
                            let by = mcu_row * v_samples[ci] + v;
                            let src_idx = by * blocks_per_row[ci] + bx;

                            let dst_idx = v * h_samples[ci] + h;
                            let block = &mut mcu_blocks[ci][dst_idx];

                            // Un-zigzag and dequantize
                            let src = &coeffs[ci][src_idx];
                            for i in 0..64 {
                                let natural = UN_ZIGZAG[i];
                                block[natural] = src[i] * qt[natural];
                            }

                            // Inverse DCT
                            unsafe { (self.idct_fn)(block) };
                        }
                    }
                }

                // Color convert and write to output
                let mcu = McuGeometry {
                    width: mcu_w,
                    height: mcu_h,
                    col: mcu_col,
                    row: mcu_row,
                };
                write_mcu_pixels(
                    self.ycbcr_to_rgb_fn,
                    &output_format,
                    &mcu_blocks,
                    sampling,
                    mcu,
                    image,
                    num_components,
                    output,
                );
            }
        }

        Ok(())
    }
}
