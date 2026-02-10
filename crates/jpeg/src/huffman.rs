//! Simple Huffman table decoder for JPEG.
//!
//! Implements the decoding algorithm from the JPEG specification (ITU-T T.81,
//! Annex F, Figure F.16). This is a straightforward bit-by-bit decoder
//! without fast lookup tables — clarity over speed.

use crate::{error::DecodeError, io::BitReader};

/// A Huffman decoding table built from a JPEG DHT marker.
///
/// For each code length (1–16), stores the maximum code value and an offset
/// into the symbol array. Decoding reads bits one at a time, checking at
/// each length whether the accumulated code matches an entry.
pub struct HuffmanTable {
    /// Maximum code value for codes of length `l` (index 1–16).
    /// Set to `-1` if no codes have that length.
    max_code: [i32; 17],

    /// Offset to convert a code value to an index into `values`.
    /// For a code of length `l` with value `code`:
    ///   `symbol = values[(code + val_offset[l]) as usize]`
    val_offset: [i32; 17],

    /// Symbol values in order of increasing code length, copied from the
    /// DHT marker data.
    values: Vec<u8>,
}

impl HuffmanTable {
    /// Build a Huffman table from DHT marker data.
    ///
    /// # Arguments
    /// * `bits`    – Number of codes for each length (index 1–16; index 0 is unused).
    /// * `symbols` – Symbol values in order of increasing code length.
    ///
    /// The codes are assigned using the canonical Huffman algorithm:
    /// within each length they are sequential, and when moving to the next
    /// length the code is shifted left by one bit.
    pub fn new(bits: &[u8; 17], symbols: [u8; 256]) -> Result<HuffmanTable, DecodeError> {
        let mut max_code = [-1i32; 17];
        let mut val_offset = [0i32; 17];

        // Total number of symbols across all code lengths
        let num_symbols: usize = bits[1..=16].iter().map(|&b| b as usize).sum();

        // Assign code values length by length
        let mut code: u32 = 0;
        let mut symbol_index: usize = 0;

        for length in 1..=16usize {
            let count = bits[length] as usize;
            if count > 0 {
                // Offset: first_symbol_index − first_code_of_this_length
                val_offset[length] = symbol_index as i32 - code as i32;
                // Last (largest) code of this length
                max_code[length] = (code + count as u32 - 1) as i32;
                code += count as u32;
                symbol_index += count;
            }
            // Codes of the next length start at double the current value
            code <<= 1;
        }

        Ok(HuffmanTable {
            max_code,
            val_offset,
            values: symbols[..num_symbols].to_vec(),
        })
    }

    /// Decode one Huffman-coded symbol from the bit stream.
    ///
    /// Reads bits one at a time, building up a code value and checking whether
    /// it matches any entry at the current length. This is the algorithm from
    /// JPEG spec Figure F.16.
    pub fn decode<R: std::io::Read>(&self, reader: &mut BitReader<R>) -> Result<u8, DecodeError> {
        let mut code: i32 = 0;

        for length in 1..=16usize {
            let bit = reader.read_bit()? as i32;
            code = (code << 1) | bit;

            if code <= self.max_code[length] {
                let index = (code + self.val_offset[length]) as usize;
                return Ok(self.values[index]);
            }
        }

        Err(DecodeError::HuffmanDecode(
            "Invalid Huffman code: no match found in 16 bits".to_string(),
        ))
    }
}
