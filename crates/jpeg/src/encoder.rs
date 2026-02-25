//! JPEG Baseline Encoder
//!
//! Encodes images to baseline JPEG format.

use jpeg_common::color_space::ColorSpace;
use jpeg_common::options::EncoderOptions;
use std::io::Write;

use crate::color_convert;
use crate::error::EncodeError;

pub mod baseline;

pub struct JpegEncoder<W: Write> {
    options: EncoderOptions,
    writer: W,
    quantization_tables: [Vec<u8>; 4],
    dc_huffman_tables: [Option<Vec<u8>>; 4],
    ac_huffman_tables: [Option<Vec<u8>>; 4],
}

impl<W: Write> JpegEncoder<W> {
    /// Create a new encoder with the given writer and options
    ///
    /// # Arguments
    /// * `writer` - The writer to which the JPEG data will be written
    /// * `options` - The options for the encoder
    pub fn new_with_options(writer: W, options: EncoderOptions) -> Result<Self, EncodeError> {
        // Validate options
        if options.width() == 0 {
            return Err(EncodeError::InvalidDimensions(
                "width must be > 0".to_string(),
            ));
        }
        if options.height() == 0 {
            return Err(EncodeError::InvalidDimensions(
                "height must be > 0".to_string(),
            ));
        }

        Ok(Self {
            options,
            writer,
            quantization_tables: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            dc_huffman_tables: [None, None, None, None],
            ac_huffman_tables: [None, None, None, None],
        })
    }

    /// Create a new encoder with default options
    pub fn new(writer: W, width: usize, height: usize) -> Result<Self, EncodeError> {
        let options = EncoderOptions::new(width, height, ColorSpace::RGB);
        Self::new_with_options(writer, options)
    }

    /// Encode the given image data to JPEG.
    ///
    /// Accepts data in any supported colorspace (Grayscale, RGB, or YCbCr).
    /// RGB input is automatically converted to YCbCr before encoding.
    ///
    /// # Arguments
    /// * `data` - The raw image data (interleaved, 1 or 3 bytes per pixel)
    pub fn encode(&mut self, data: &[u8]) -> Result<(), EncodeError> {
        match self.options.colorspace() {
            ColorSpace::Grayscale | ColorSpace::YCbCr => {
                baseline::encode_baseline(&mut self.writer, &self.options, data)
            }
            ColorSpace::RGB => {
                // Convert RGB → YCbCr, then encode as YCbCr.
                let mut ycbcr = vec![0u8; data.len()];
                color_convert::scalar::rgb_to_ycbcr(data, &mut ycbcr);
                let opts = self.options.clone().set_colorspace(ColorSpace::YCbCr);
                baseline::encode_baseline(&mut self.writer, &opts, &ycbcr)
            }
            _ => Err(EncodeError::UnsupportedColorSpace),
        }
    }
}
