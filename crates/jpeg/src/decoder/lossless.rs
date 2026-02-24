//! Lossless JPEG decoding (SOF3).
//!
//! Lossless JPEG is fundamentally different from the DCT-based modes:
//!
//! - No DCT, no quantization tables, no zigzag ordering.
//! - Each sample is predicted from its neighbours using one of 7 predictors.
//! - The prediction residual (difference) is Huffman-coded using the DC table.
//! - The SOS `Ss` byte selects the predictor (1–7).
//! - The SOS `Al` byte is the point transform (right-shift before encoding).
//!
//! Reference: ITU-T T.81, Section H (Lossless mode of operation).

use crate::{error::DecodeError, io::BitReader};

use super::{JpegDecoder, baseline::receive_extend};

// ── Predictor functions ─────────────────────────────────────────────────────

/// Compute the predicted value for a sample at position (x, y).
///
/// The three neighbours used are:
/// - `a` = sample to the left   (x-1, y)
/// - `b` = sample above          (x, y-1)
/// - `c` = sample above-left     (x-1, y-1)
///
/// Predictor selection values (from SOS Ss byte):
///  1: Px = a
///  2: Px = b
///  3: Px = c
///  4: Px = a + b - c
///  5: Px = a + (b - c) / 2
///  6: Px = b + (a - c) / 2
///  7: Px = (a + b) / 2
#[inline]
fn predict(predictor: u8, a: i32, b: i32, c: i32) -> i32 {
    match predictor {
        1 => a,
        2 => b,
        3 => c,
        4 => a + b - c,
        5 => a + ((b - c) >> 1),
        6 => b + ((a - c) >> 1),
        7 => (a + b) >> 1,
        _ => unreachable!("invalid predictor selection {predictor}"),
    }
}

// ── Decode entry point ──────────────────────────────────────────────────────

impl<R: std::io::Read> JpegDecoder<R> {
    /// Decode a lossless JPEG (SOF3) image into `output`.
    pub(crate) fn decode_lossless(&mut self, output: &mut [u8]) -> Result<(), DecodeError> {
        let width = self.info.width;
        let height = self.info.height;
        let precision = self.info.precision;
        let num_components = self.info.components;

        // Lossless mode: Ss selects the predictor (1–7), Al is the point transform
        let predictor = self.scan_ss;
        let point_transform = self.scan_al;

        if !(1..=7).contains(&predictor) {
            return Err(DecodeError::Format(format!(
                "Invalid lossless predictor selection: {predictor} (must be 1–7)"
            )));
        }

        // Validate DC Huffman tables exist for all components in the scan
        let scan_indices = self.scan_component_indices.clone();
        let dc_ht_ids: Vec<usize> = scan_indices
            .iter()
            .map(|&ci| self.components[ci].dc_huffman_table_id)
            .collect();

        for (i, &ht_id) in dc_ht_ids.iter().enumerate() {
            if self.dc_huffman_tables[ht_id].is_none() {
                return Err(DecodeError::Format(format!(
                    "Scan component {} references missing DC Huffman table {}",
                    scan_indices[i], ht_id
                )));
            }
        }

        let bytes_per_sample: usize = if precision > 8 { 2 } else { 1 };
        let row_stride = width * num_components * bytes_per_sample;

        // Initial prediction value: 2^(P-Pt-1) where P = precision, Pt = point transform
        let initial_pred = 1i32 << (precision as i32 - point_transform as i32 - 1);

        // Maximum sample value before point transform restoration
        let max_val = (1i32 << precision) - 1;

        // Allocate a working buffer of i32 samples for the current and previous row.
        // Each row has width * num_components samples.
        let row_samples = width * num_components;
        let mut prev_row = vec![0i32; row_samples];
        let mut cur_row = vec![0i32; row_samples];

        // Restart tracking
        let restart_interval = self.restart_interval;
        let mut sample_count: usize = 0;

        let mut reader = BitReader::new(&mut self.reader);

        for y in 0..height {
            for x in 0..width {
                // Handle restart markers (in lossless mode, the restart counter
                // counts samples rather than MCUs when each MCU is 1 sample).
                if restart_interval > 0 && sample_count > 0 && sample_count % restart_interval == 0
                {
                    let marker = reader.read_restart_marker()?;
                    if !matches!(marker, 0xD0..=0xD7) {
                        return Err(DecodeError::Format(format!(
                            "Expected RST marker, found 0xFF{marker:02X}"
                        )));
                    }
                }

                for ci in 0..num_components {
                    let ht_id = dc_ht_ids[ci];
                    let dc_table = self.dc_huffman_tables[ht_id].as_ref().unwrap();

                    // Determine neighbour values for prediction (a=left, b=above, c=above-left).
                    let idx = x * num_components + ci;
                    let a = if x > 0 {
                        cur_row[idx - num_components]
                    } else if y > 0 {
                        prev_row[idx]
                    } else {
                        initial_pred
                    };
                    let b = if y > 0 { prev_row[idx] } else { a };
                    let c = if x > 0 && y > 0 {
                        prev_row[idx - num_components]
                    } else {
                        a
                    };

                    // For the first row, predictor is forced to 1 (left) per spec H.1.1
                    // For the first column, predictor is forced to 2 (above) per spec H.1.1
                    // For pixel (0,0), use initial_pred (handled above via `a`)
                    let eff_predictor = if y == 0 && x == 0 {
                        // (0,0): a is already initial_pred, use predictor 1
                        1
                    } else if y == 0 {
                        // First row: always predict from left
                        1
                    } else if x == 0 {
                        // First column: always predict from above
                        2
                    } else {
                        predictor
                    };

                    let px = predict(eff_predictor, a, b, c);

                    // Decode the Huffman-coded difference
                    let category = dc_table.decode(&mut reader)?;
                    let diff = if category > 0 {
                        let bits = reader.read_bits(category)?;
                        receive_extend(bits, category)
                    } else {
                        0
                    };

                    // Reconstruct the sample value
                    let mut sample = px + diff;

                    // Apply inverse point transform (left-shift by Al)
                    if point_transform > 0 {
                        sample <<= point_transform;
                    }

                    // Clamp to valid range
                    sample = sample.clamp(0, max_val);

                    // Store the reconstructed (shifted and clamped) value for prediction
                    // of subsequent samples. Per T.81 H.1.2.3, prediction uses Rx values.
                    cur_row[idx] = sample;

                    // Write to output buffer
                    let out_offset = y * row_stride
                        + x * num_components * bytes_per_sample
                        + ci * bytes_per_sample;
                    if bytes_per_sample == 2 {
                        let val = sample as u16;
                        output[out_offset] = val as u8;
                        output[out_offset + 1] = (val >> 8) as u8;
                    } else {
                        output[out_offset] = sample as u8;
                    }
                }

                sample_count += 1;
            }

            // Swap rows
            std::mem::swap(&mut prev_row, &mut cur_row);
        }

        Ok(())
    }
}
