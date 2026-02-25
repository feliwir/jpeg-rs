use crate::error::EncodeError;

/// A bit writer for JPEG entropy-coded data.
///
/// Writes bits MSB-first into the underlying byte stream.
/// Performs JPEG byte stuffing: whenever a `0xFF` byte is emitted,
/// a `0x00` stuff byte is appended to distinguish data from markers.
pub struct BitWriter<W> {
    writer: W,
    /// Accumulated bits, MSB-aligned.  The next bit to be written goes
    /// into position `7 - bits_used` of the current byte.
    bit_buffer: u32,
    /// Number of bits currently buffered (0–31).
    bits_in_buffer: u8,
}

impl<W: std::io::Write> BitWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            bit_buffer: 0,
            bits_in_buffer: 0,
        }
    }

    /// Write `n` bits (1–24) to the stream, MSB first.
    pub fn write_bits(&mut self, value: u32, n: u8) -> Result<(), EncodeError> {
        debug_assert!(n > 0 && n <= 24);
        // Shift the value into the upper part of the buffer
        self.bit_buffer = (self.bit_buffer << n) | (value & ((1 << n) - 1));
        self.bits_in_buffer += n;

        // Flush complete bytes
        while self.bits_in_buffer >= 8 {
            self.bits_in_buffer -= 8;
            let byte = ((self.bit_buffer >> self.bits_in_buffer) & 0xFF) as u8;
            self.writer.write_all(&[byte])?;

            // JPEG byte stuffing: 0xFF data → 0xFF 0x00
            if byte == 0xFF {
                self.writer.write_all(&[0x00])?;
            }
        }

        Ok(())
    }

    /// Flush any remaining bits, padding with 1-bits to reach a byte boundary.
    ///
    /// JPEG pads with 1-bits so that the padding cannot be mistaken for
    /// a valid Huffman code (all-ones is never a valid prefix).
    pub fn flush(&mut self) -> Result<(), EncodeError> {
        if self.bits_in_buffer > 0 {
            let pad = 8 - self.bits_in_buffer;
            // Pad with 1-bits
            self.write_bits((1 << pad) - 1, pad)?;
        }
        Ok(())
    }

    /// Get a mutable reference to the inner writer.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_write_bits_basic() {
        let mut buf = Vec::new();
        let mut bw = BitWriter::new(Cursor::new(&mut buf));
        bw.write_bits(0b1010, 4).unwrap();
        bw.write_bits(0b1100, 4).unwrap();
        bw.flush().unwrap();
        let inner = bw.get_mut().get_mut();
        assert_eq!(*inner, &[0b10101100]);
    }

    #[test]
    fn test_byte_stuffing() {
        let mut buf = Vec::new();
        let mut bw = BitWriter::new(Cursor::new(&mut buf));
        bw.write_bits(0xFF, 8).unwrap();
        bw.write_bits(0x42, 8).unwrap();
        bw.flush().unwrap();
        let inner = bw.get_mut().get_mut();
        // 0xFF should be followed by 0x00 stuff byte
        assert_eq!(*inner, &[0xFF, 0x00, 0x42]);
    }

    #[test]
    fn test_flush_padding() {
        let mut buf = Vec::new();
        let mut bw = BitWriter::new(Cursor::new(&mut buf));
        bw.write_bits(0b101, 3).unwrap();
        bw.flush().unwrap();
        let inner = bw.get_mut().get_mut();
        // 101 + 11111 padding = 10111111 = 0xBF
        assert_eq!(*inner, &[0xBF]);
    }
}
