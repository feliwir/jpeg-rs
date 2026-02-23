use jpeg_common::{color_space::ColorSpace, options::DecoderOptions};
use std::io::BufRead;

use crate::{
    color_convert,
    component::{Component, MAX_COMPONENTS},
    error::DecodeError,
    huffman::HuffmanTable,
    idct,
    marker::{self, MARKER_PREFIX, Marker},
};

mod mcu;

#[derive(Default)]
pub struct ImageInfo {
    /// The width of the image in pixels
    pub width: usize,
    /// The height of the image in pixels
    pub height: usize,
    /// The precision of the image in bits
    pub precision: u8,
    /// The number of components in the image
    pub components: usize,
    /// The sub sampling ratio of the image (horizontal, vertical)
    pub sample_ratio: (u8, u8),
}

pub struct JpegDecoder<R> {
    pub(crate) info: ImageInfo,
    headers_decoded: bool,
    // Header decoding properties
    pub(crate) is_progressive: bool,
    pub(crate) did_read_sof: bool,
    pub(crate) input_colorspace: ColorSpace,
    pub(crate) components: Vec<Component>,
    pub(crate) quantization_tables: [Option<[i32; 64]>; MAX_COMPONENTS],
    pub(crate) dc_huffman_tables: [Option<HuffmanTable>; MAX_COMPONENTS],
    pub(crate) ac_huffman_tables: [Option<HuffmanTable>; MAX_COMPONENTS],
    pub(crate) h_max: usize,
    pub(crate) v_max: usize,
    pub(crate) mcu_width: usize,
    pub(crate) mcu_height: usize,
    pub(crate) mcu_x: usize,
    pub(crate) mcu_y: usize,
    pub(crate) num_scans: usize,
    pub(crate) z_order: [usize; MAX_COMPONENTS],
    /// Restart interval (number of MCUs between RST markers). 0 = no restarts.
    pub(crate) restart_interval: usize,
    // The options for the decoder
    options: DecoderOptions,
    // The reader from which the jpeg data will be read
    pub(crate) reader: R,
    // Delegate functions
    pub(crate) idct_fn: unsafe fn(&mut [i32; 64]),
    pub(crate) ycbcr_to_rgb_fn: fn(&[i16; 16], &[i16; 16], &[i16; 16], &mut [u8], &mut usize),
}

impl<R: BufRead> JpegDecoder<R> {
    /// Create a new decoder with the given reader and options
    ///
    /// # Arguments
    /// * `reader` - The reader from which the jpeg data will be read
    /// * `options` - The options for the decoder
    pub fn new_with_options(reader: R, options: DecoderOptions) -> Self {
        Self {
            reader,
            options,
            info: ImageInfo::default(),
            headers_decoded: false,
            is_progressive: false,
            did_read_sof: false,
            input_colorspace: ColorSpace::Unknown,
            components: Vec::new(),
            quantization_tables: [None, None, None, None],
            dc_huffman_tables: [None, None, None, None],
            ac_huffman_tables: [None, None, None, None],
            h_max: 1,
            v_max: 1,
            mcu_width: 0,
            mcu_height: 0,
            mcu_x: 0,
            mcu_y: 0,
            num_scans: 0,
            z_order: [0; MAX_COMPONENTS],
            restart_interval: 0,
            idct_fn: idct::select_idct_fn(8, options.forced_simd_backend()),
            ycbcr_to_rgb_fn: color_convert::select_ycbcr_to_rgb_converter(),
        }
    }

    /// Create a new decoder with the given reader and default options
    ///
    /// # Arguments
    /// * `reader` - The reader from which the jpeg data will be read
    pub fn new(reader: R) -> Self {
        Self::new_with_options(reader, DecoderOptions::default())
    }

    /// Decode only the headers
    ///
    /// This can be used to get the image information without decoding the entire image.
    ///
    /// # Returns
    /// * `Ok(())` if the headers were decoded successfully
    /// * `Err(DecodeError)` if there was an error decoding the headers
    pub fn decode_headers(&mut self) -> Result<(), DecodeError> {
        if self.headers_decoded {
            log::trace!("Headers already decoded, skipping");
            return Ok(());
        }

        self.expect_marker(Marker::SOI)?;

        // Read segments until we find the SOS marker
        'decode_markers: loop {
            let marker = self.read_marker()?;
            match marker {
                // Start of Frame markers
                Marker::SOF(0..=2) => {
                    // choose marker
                    if marker == Marker::SOF(0) || marker == Marker::SOF(1) {
                    } else {
                        self.is_progressive = true;
                    }

                    log::trace!("Image encoding scheme =`{:?}`", marker);
                    marker::read_start_of_frame(self, marker)?;

                    // Check if we need to switch the IDCT function based on the precision (if it's not 8 bits per sample, we need to use a different IDCT function)
                    if self.info.precision != 8 {
                        self.idct_fn = idct::select_idct_fn(
                            self.info.precision,
                            self.options.forced_simd_backend(),
                        );
                    }
                }
                Marker::SOF(v) => {
                    return Err(DecodeError::Unsupported(format!(
                        "Unsupported SOF marker found: SOF({v})"
                    )));
                }
                // APP markers are used for metadata and can be skipped
                Marker::APP(0) => {
                    log::trace!("Found JFIF APP0 segment, skipping");
                    let length = self.read_length()?;
                    self.skip_segment(length as u16)?;
                }
                Marker::APP(v) => {
                    log::trace!("Found APP{}, skipping", v);
                    let length = self.read_length()?;
                    self.skip_segment(length as u16)?;
                }
                // Start of Scan indicates the end of the headers and the start of the compressed image data
                Marker::SOS => {
                    log::trace!(
                        "Found SOS segment, reading scan header and stopping header decoding"
                    );
                    marker::read_start_of_scan(self)?;
                    self.headers_decoded = true;
                    break 'decode_markers;
                }
                // Parse the quantization tables, but we will not use them until we decode the image data
                Marker::DQT => {
                    log::trace!("Found DQT segment, reading quantization tables");
                    marker::read_quant_tables(self)?;
                }
                // Parse the huffman tables, but we will not use them until we decode the image data
                Marker::DHT => {
                    log::trace!("Found DHT segment, reading huffman tables");
                    marker::read_huffman_tables(self)?;
                }
                // Parse the restart interval
                Marker::DRI => {
                    log::trace!("Found DRI segment, reading restart interval");
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
                    log::trace!("Restart interval: {} MCUs", self.restart_interval);
                }
                // End of Image found before Start of Scan
                Marker::EOI => {
                    return Err(DecodeError::FormatStatic(
                        "EOI marker found before SOS marker",
                    ));
                }
                // Skip other segments
                _ => {
                    let length = self.read_length()?;
                    self.skip_segment(length as u16)?;
                }
            }
        }
        Ok(())
    }

    /// Get the image information
    ///
    /// This will return `None` if the headers have not been decoded yet
    /// Call `decode_headers` or `decode` to decode the headers before calling this method
    ///
    /// # Returns
    /// * `Some(&ImageInfo)` containing the image information if the headers have been decoded
    /// * `None` if the headers have not been decoded yet
    #[must_use]
    pub fn info(&self) -> Option<&ImageInfo> {
        if !self.headers_decoded {
            return None;
        }
        Some(&self.info)
    }

    /// Calculate the requried buffer size for the decoded image data based on the image information
    ///
    /// This will return `None` if the headers have not been decoded yet
    /// Call `decode_headers` or `decode` to decode the headers before calling this method
    ///
    /// # Returns
    /// * `Some(usize)` containing the required buffer size for the decoded image data if the headers have been decoded
    /// * `None` if the headers have not been decoded yet
    #[must_use]
    pub fn required_buffer_size(&self) -> Option<usize> {
        if !self.headers_decoded {
            return None;
        }
        let color_space = self
            .options
            .out_colorspace()
            .unwrap_or(match self.info.components {
                1 => ColorSpace::Grayscale,
                3 => ColorSpace::YCbCr,
                4 => ColorSpace::CMYK,
                _ => return None,
            });
        let components = color_space.num_components();
        let bit_depth = if self.info.precision > 8 { 2 } else { 1 };
        Some(self.info.width * self.info.height * components * bit_depth)
    }

    /// Decode the JPEG data from the reader and return the decoded pixel data as a vector of bytes
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` containing the decoded pixel data if the decoding was successful
    /// * `Err(DecodeError)` if there was an error during decoding
    pub fn decode(&mut self) -> Result<Vec<u8>, DecodeError> {
        self.decode_headers()?;

        let buffer_size = self
            .required_buffer_size()
            .ok_or_else(|| DecodeError::FormatStatic("Could not calculate required buffer size"))?;
        let mut buffer = vec![0u8; buffer_size];
        self.decode_into(&mut buffer)?;

        Ok(buffer)
    }

    /// Decode the JPEG data into an already allocated buffer.
    /// The buffer must be large enough to hold the decoded pixel data.
    ///
    /// # Arguments
    /// * `buffer` - The buffer into which the decoded pixel data will be written
    ///
    /// # Returns
    /// * `Ok(())` if the decoding was successful
    /// * `Err(DecodeError)` if there was an error during decoding
    pub fn decode_into(&mut self, buffer: &mut [u8]) -> Result<(), DecodeError> {
        self.decode_headers()?;

        let expected_size = self
            .required_buffer_size()
            .ok_or_else(|| DecodeError::FormatStatic("Could not calculate required buffer size"))?;
        if buffer.len() < expected_size {
            return Err(DecodeError::Format(format!(
                "Buffer too small for decoded image data: expected at least {} bytes, got {} bytes",
                expected_size,
                buffer.len()
            )));
        }

        if self.is_progressive {
            return Err(DecodeError::Unsupported(
                "Progressive JPEG decoding is not supported yet".to_string(),
            ));
        } else {
            self.decode_mcu_ycbcr(buffer)?;
        }

        Ok(())
    }

    /// Read the length of a segment from the reader
    ///
    /// This should be called after reading a marker that has a length field (most markers except RST, SOI, EOI, TEM)
    /// The length includes the 2 bytes of the length field itself, so the actual data length is `length - 2`
    ///
    /// # Returns
    /// * `Ok(usize)` containing the length of the segment if it was read successfully
    /// * `Err(DecodeError)` if there was an error reading the length
    pub(crate) fn read_length(&mut self) -> Result<usize, DecodeError> {
        let mut buf = [0u8; 2];
        self.reader.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf) as usize)
    }

    /// Skip a segment of the given length
    ///
    /// This should be called after reading a marker that has a length field (most markers except RST, SOI, EOI, TEM) and reading the length of the segment
    /// The length includes the 2 bytes of the length field itself, so the actual data length is `length - 2`
    ///
    /// # Arguments
    /// * `length` - The length of the segment to skip, including the 2 bytes of the length field itself
    ///
    /// # Returns
    /// * `Ok(())` if the segment was skipped successfully
    /// * `Err(DecodeError)` if there was an error skipping the segment
    pub(crate) fn skip_segment(&mut self, length: u16) -> Result<(), DecodeError> {
        let skip_bytes = length as usize - 2;
        let mut buf = vec![0u8; skip_bytes];
        self.reader.read_exact(&mut buf)?;
        Ok(())
    }

    /// Expect the next marker to be the given marker and return an error if it is not
    ///
    /// # Arguments
    /// * `expected` - The marker that is expected to be read next
    ///
    /// # Returns
    /// * `Ok(())` if the expected marker was read successfully
    /// * `Err(DecodeError)` if the expected marker was not read
    pub(crate) fn expect_marker(&mut self, expected: Marker) -> Result<(), DecodeError> {
        let marker = self.read_marker()?;
        if marker != expected {
            return Err(DecodeError::InvalidMarker(
                ((MARKER_PREFIX as u16) << 8) | marker.into_u8().unwrap_or(0) as u16,
            ));
        }
        Ok(())
    }

    /// Read the next marker from the reader and return it
    ///
    /// This will skip any fill bytes (0xFF) and any skippable markers (APPn, COM) until it finds a valid marker that is not skippable
    ///
    /// # Returns
    /// * `Ok(Marker)` containing the next marker if it was read successfully
    /// * `Err(DecodeError)` if there was an error reading the marker
    pub(crate) fn read_marker(&mut self) -> Result<Marker, DecodeError> {
        let mut buf = [0u8; 2];
        self.reader.read_exact(&mut buf)?;

        if buf[0] != MARKER_PREFIX {
            return Err(DecodeError::InvalidMarker(u16::from_be_bytes(buf)));
        }

        // Skip fill bytes (0xFF)
        let mut marker_byte = buf[1];
        while marker_byte == MARKER_PREFIX {
            let mut byte = [0u8; 1];
            self.reader.read_exact(&mut byte)?;
            marker_byte = byte[0];
        }

        // Try to convert to known marker
        if let Some(m) = Marker::from_u8(marker_byte) {
            return Ok(m);
        }

        // Handle unknown but skippable markers: APP markers (0xE0-0xEF) and COM (0xFE)
        if (marker_byte >= 0xE0 && marker_byte <= 0xEF) || marker_byte == 0xFE {
            // Read and skip the segment
            let length = self.read_length()?;
            self.skip_segment(length as u16)?;
            // Recursively read the next marker
            return self.read_marker();
        }

        Err(DecodeError::InvalidMarker(u16::from_be_bytes([
            MARKER_PREFIX,
            marker_byte,
        ])))
    }
}
