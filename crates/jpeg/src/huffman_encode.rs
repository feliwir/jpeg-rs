//! Huffman encoding table for JPEG.
//!
//! Builds a lookup table from symbol → (code, length) using the canonical
//! Huffman assignment algorithm (ITU-T T.81, Annex C).

use crate::error::EncodeError;
use crate::io::BitWriter;

/// A Huffman encoding table: maps each symbol (0–255) to its code and length.
pub struct HuffmanEncodeTable {
    /// Huffman code for each symbol (up to 16 bits, right-aligned).
    codes: [u16; 256],
    /// Code length in bits for each symbol (0 = symbol not in table).
    lengths: [u8; 256],
}

impl HuffmanEncodeTable {
    /// Build an encoding table from the DHT-style `lengths` and `values` arrays.
    ///
    /// * `lengths` – 16 entries: number of codes of length 1, 2, …, 16.
    /// * `values`  – symbol values in order of increasing code length.
    pub fn new(lengths: &[u8; 16], values: &[u8]) -> Self {
        let mut codes = [0u16; 256];
        let mut code_lengths = [0u8; 256];

        // Generate canonical Huffman codes (JPEG Annex C, Figure C.1–C.3)
        let mut code: u16 = 0;
        let mut si = 0usize; // symbol index into `values`

        for bits in 1..=16u8 {
            let count = lengths[(bits - 1) as usize] as usize;
            for _ in 0..count {
                let symbol = values[si] as usize;
                codes[symbol] = code;
                code_lengths[symbol] = bits;
                si += 1;
                code += 1;
            }
            code <<= 1;
        }

        Self {
            codes,
            lengths: code_lengths,
        }
    }

    /// Encode a single symbol, writing its Huffman code to the bit writer.
    #[inline]
    pub fn encode<W: std::io::Write>(
        &self,
        writer: &mut BitWriter<W>,
        symbol: u8,
    ) -> Result<(), EncodeError> {
        let code = self.codes[symbol as usize];
        let len = self.lengths[symbol as usize];
        debug_assert!(len > 0, "Symbol {symbol} has no Huffman code");
        writer.write_bits(code as u32, len)
    }
}

/// Compute the number of bits needed to represent `value` and the
/// corresponding bit pattern for Huffman encoding (JPEG Annex F, Figure F.1).
///
/// Returns `(category, bits)`:
/// - `category` (SSSS): number of additional bits (0–11)
/// - `bits`: the additional bits to append after the Huffman code
#[inline]
pub fn encode_coefficient(value: i32) -> (u8, u16) {
    if value == 0 {
        return (0, 0);
    }

    let abs = value.unsigned_abs();
    let category = 32 - abs.leading_zeros() as u8; // ceil(log2(abs+1))

    // For positive values, bits = value.
    // For negative values, bits = value - 1 (ones' complement).
    let bits = if value > 0 {
        value as u16
    } else {
        (value - 1 + (1 << category)) as u16
    };

    (category, bits)
}
