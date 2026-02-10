use crate::error::DecodeError;

/// A bit reader for JPEG entropy-coded data.
///
/// Reads bits MSB-first from the underlying byte stream.
/// Handles JPEG byte stuffing: in the entropy-coded segment, a `0xFF` data byte
/// is always followed by a stuffed `0x00` byte (which is discarded) to distinguish
/// data from markers.
pub struct BitReader<R> {
    reader: R,
    /// Accumulated bits. The `bits_left` most-significant valid bits are stored
    /// right-aligned (i.e. in positions `bits_left-1` down to `0`).
    bit_buffer: u32,
    /// Number of valid bits currently in `bit_buffer`.
    bits_left: u8,
}

impl<R: std::io::Read> BitReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            bit_buffer: 0,
            bits_left: 0,
        }
    }

    /// Read the next data byte, handling JPEG byte stuffing.
    ///
    /// In JPEG entropy-coded data:
    /// - `0xFF 0x00` → data byte `0xFF` (the `0x00` stuff byte is removed)
    /// - `0xFF <non-zero>` → this is a marker, which is an error here
    fn read_byte_stuffed(&mut self) -> Result<u8, DecodeError> {
        let mut buf = [0u8; 1];
        self.reader.read_exact(&mut buf)?;
        let byte = buf[0];

        if byte != 0xFF {
            return Ok(byte);
        }

        // Got 0xFF — peek at the next byte
        self.reader.read_exact(&mut buf)?;
        match buf[0] {
            0x00 => Ok(0xFF), // Byte stuffing: 0xFF 0x00 → 0xFF
            marker => Err(DecodeError::Format(format!(
                "Unexpected marker 0xFF{marker:02X} in entropy-coded data"
            ))),
        }
    }

    /// Ensure at least `needed` bits are available in the buffer.
    fn fill_bits(&mut self, needed: u8) -> Result<(), DecodeError> {
        while self.bits_left < needed {
            let byte = self.read_byte_stuffed()? as u32;
            self.bit_buffer = (self.bit_buffer << 8) | byte;
            self.bits_left += 8;
        }
        Ok(())
    }

    /// Read `n` bits (1–16) from the stream, MSB first.
    pub fn read_bits(&mut self, n: u8) -> Result<u16, DecodeError> {
        debug_assert!(n > 0 && n <= 16);
        self.fill_bits(n)?;
        self.bits_left -= n;
        let result = (self.bit_buffer >> self.bits_left) & ((1 << n) - 1);
        Ok(result as u16)
    }

    /// Read a single bit from the stream. Returns 0 or 1.
    pub fn read_bit(&mut self) -> Result<u16, DecodeError> {
        self.read_bits(1)
    }

    /// Discard all buffered bits, then read two raw bytes to consume a restart
    /// marker.  Returns the second byte (the marker identifier, e.g. `0xD0`).
    ///
    /// Restart markers (`0xFF 0xD0`–`0xFF 0xD7`) appear between MCU groups
    /// at byte-aligned positions.  Any padding bits left in the buffer from
    /// the previous segment are discarded.
    pub fn read_restart_marker(&mut self) -> Result<u8, DecodeError> {
        self.bits_left = 0;
        self.bit_buffer = 0;
        let mut buf = [0u8; 2];
        self.reader.read_exact(&mut buf)?;
        if buf[0] != 0xFF {
            return Err(DecodeError::Format(format!(
                "Expected restart marker, found 0x{:02X}{:02X}",
                buf[0], buf[1]
            )));
        }
        Ok(buf[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_bits() {
        let data = vec![0b10101010, 0b11001100];
        let mut reader = BitReader::new(Cursor::new(data));
        assert_eq!(reader.read_bits(4).unwrap(), 0b1010);
        assert_eq!(reader.read_bits(4).unwrap(), 0b1010);
        assert_eq!(reader.read_bits(4).unwrap(), 0b1100);
        assert_eq!(reader.read_bits(4).unwrap(), 0b1100);
    }

    #[test]
    fn test_byte_stuffing() {
        // 0xFF 0x00 should be read as a single 0xFF data byte
        let data = vec![0xFF, 0x00, 0x42];
        let mut reader = BitReader::new(Cursor::new(data));
        assert_eq!(reader.read_bits(8).unwrap(), 0xFF);
        assert_eq!(reader.read_bits(8).unwrap(), 0x42);
    }

    #[test]
    fn test_single_bits() {
        let data = vec![0b10110001];
        let mut reader = BitReader::new(Cursor::new(data));
        assert_eq!(reader.read_bit().unwrap(), 1);
        assert_eq!(reader.read_bit().unwrap(), 0);
        assert_eq!(reader.read_bit().unwrap(), 1);
        assert_eq!(reader.read_bit().unwrap(), 1);
        assert_eq!(reader.read_bit().unwrap(), 0);
        assert_eq!(reader.read_bit().unwrap(), 0);
        assert_eq!(reader.read_bit().unwrap(), 0);
        assert_eq!(reader.read_bit().unwrap(), 1);
    }
}
