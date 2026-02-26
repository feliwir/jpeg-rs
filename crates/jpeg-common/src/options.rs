use crate::color_space::ColorSpace;

#[derive(Debug, Copy, Clone)]
pub enum SimdBackend {
    Scalar,
    #[allow(dead_code)]
    Neon,
    #[allow(dead_code)]
    Sse,
    #[allow(dead_code)]
    Avx2,
    #[allow(dead_code)]
    Avx512,
}

impl SimdBackend {
    pub fn is_supported(self) -> bool {
        match self {
            SimdBackend::Scalar => true,
            #[cfg(target_arch = "aarch64")]
            SimdBackend::Neon => true,
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdBackend::Sse => is_x86_feature_detected!("sse4.1"),
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdBackend::Avx2 => is_x86_feature_detected!("avx2"),
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdBackend::Avx512 => {
                is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512dq")
            }
            _ => false,
        }
    }

    pub fn iter() -> impl Iterator<Item = SimdBackend> {
        let mut backends = vec![SimdBackend::Scalar];
        #[cfg(target_arch = "aarch64")]
        {
            backends.push(SimdBackend::Neon);
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            backends.push(SimdBackend::Sse);
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            backends.push(SimdBackend::Avx2);
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            backends.push(SimdBackend::Avx512);
        }
        backends.into_iter()
    }
}

impl std::fmt::Display for SimdBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimdBackend::Scalar => write!(f, "scalar"),
            SimdBackend::Neon => write!(f, "neon"),
            SimdBackend::Sse => write!(f, "sse"),
            SimdBackend::Avx2 => write!(f, "avx2"),
            SimdBackend::Avx512 => write!(f, "avx512"),
        }
    }
}

/// Decoder options
///
/// Not all options are respected by all decoders
#[derive(Debug, Copy, Clone)]
pub struct DecoderOptions {
    /// Maximum width for which decoders will
    /// not try to decode images larger than
    /// the specified width.
    ///
    /// - Default value: 16384
    /// - Respected by: `all decoders`
    max_width: usize,
    /// Maximum height for which decoders will not
    /// try to decode images larger than the
    /// specified height
    ///
    /// - Default value: 16384
    /// - Respected by: `all decoders`
    max_height: usize,
    /// Output colorspace
    ///
    /// The jpeg decoder allows conversion to a separate colorspace
    /// than the input. If None, the output colorspace will be the same as the input colorspace.
    ///
    /// I.e you can convert a RGB jpeg image to grayscale without
    /// first decoding it to RGB to get
    ///
    /// - Default value: `None`
    /// - Respected by: `jpeg`
    out_colorspace: Option<ColorSpace>,

    /// SIMD backend to use for decoding
    /// - Default value: `SimdBackend::Scalar`
    /// - Respected by: `jpeg`
    forced_simd_backend: Option<SimdBackend>,
}

impl Default for DecoderOptions {
    fn default() -> Self {
        Self {
            max_width: 16384,
            max_height: 16384,
            out_colorspace: None,
            forced_simd_backend: None,
        }
    }
}

impl DecoderOptions {
    /// Get the maximum width
    pub fn max_width(&self) -> usize {
        self.max_width
    }

    /// Set the maximum width
    ///
    /// # Arguments
    /// * `max_width` - The maximum width for which decoders will not try to decode
    /// images larger than the specified width.
    pub fn set_max_width(mut self, max_width: usize) -> Self {
        self.max_width = max_width;
        self
    }

    /// Get the maximum height
    pub fn max_height(&self) -> usize {
        self.max_height
    }

    /// Set the maximum height
    ///     
    /// # Arguments
    /// * `max_height` - The maximum height for which decoders will not try to decode
    /// images larger than the specified height.
    pub fn set_max_height(mut self, max_height: usize) -> Self {
        self.max_height = max_height;
        self
    }

    /// Get the output colorspace
    pub fn out_colorspace(&self) -> Option<ColorSpace> {
        self.out_colorspace
    }

    /// Set the output colorspace
    ///
    /// # Arguments
    /// * `out_colorspace` - The output colorspace for the decoder.
    /// If None, the output colorspace will be the same as the input colorspace.
    pub fn set_out_colorspace(mut self, out_colorspace: Option<ColorSpace>) -> Self {
        self.out_colorspace = out_colorspace;
        self
    }

    /// Get the forced SIMD backend
    ///
    /// If None, the decoder will choose the best available SIMD backend for the current platform.
    pub fn forced_simd_backend(&self) -> Option<SimdBackend> {
        self.forced_simd_backend
    }

    /// Set the forced SIMD backend
    ///
    /// # Arguments
    /// * `forced_simd_backend` - The SIMD backend to use for decoding.
    /// If None, the decoder will choose the best available SIMD backend for the current platform.
    pub fn set_forced_simd_backend(mut self, forced_simd_backend: Option<SimdBackend>) -> Self {
        self.forced_simd_backend = forced_simd_backend;
        self
    }
}

/// Encoder options
///
/// Not all options are respected by all encoders
#[derive(Debug, Copy, Clone)]
pub struct EncoderOptions {
    /// Quality for lossy encoders
    ///
    /// - Default value: 85
    /// - Respected by: `jpeg`
    quality: u8,

    /// Width of the image
    ///
    /// - Default value: 0 (must be set by the user for encoders that require it)
    /// - Respected by: `jpeg`
    width: usize,

    /// Height of the image
    ///
    /// - Default value: 0 (must be set by the user for encoders that require it)
    /// - Respected by: `jpeg`
    height: usize,

    /// Precision of the input image (bits per channel)
    ///
    /// - Default value: 8 (must be set by the user for encoders that require it)
    /// - Respected by: `jpeg`
    precision: u8,

    /// Colorspace of the input image
    ///
    /// - Default value: `RGB`
    /// - Respected by: `jpeg`
    colorspace: ColorSpace,

    /// Chroma subsampling for the encoder
    ///
    /// - Default value: `YCbCr 4:2:0`
    /// - Respected by: `jpeg`
    chroma_subsampling: (u8, u8),

    /// Progressive encoding for the encoder
    ///
    /// - Default value: `false`
    /// - Respected by: `jpeg`
    progressive: bool,

    /// Lossless encoding for the encoder
    ///
    /// - Default value: `false`
    /// - Respected by: `jpeg`
    lossless: bool,

    /// SIMD backend to use for encoding
    ///
    /// - Default value: `None`
    /// - Respected by: `jpeg`
    forced_simd_backend: Option<SimdBackend>,
}

impl Default for EncoderOptions {
    fn default() -> Self {
        Self {
            quality: 85,
            width: 0,
            height: 0,
            precision: 8,
            colorspace: ColorSpace::RGB,
            chroma_subsampling: (4, 2),
            progressive: false,
            lossless: false,
            forced_simd_backend: None,
        }
    }
}

impl EncoderOptions {
    pub fn new(width: usize, height: usize, colorspace: ColorSpace) -> Self {
        Self {
            width,
            height,
            colorspace,
            ..Self::default()
        }
    }

    /// Get the quality for lossy encoders
    pub fn quality(&self) -> u8 {
        self.quality
    }

    /// Set the quality for lossy encoders
    ///
    /// # Arguments
    /// * `quality` - The quality for lossy encoders (0-100).
    pub fn set_quality(mut self, quality: u8) -> Self {
        self.quality = quality;
        self
    }

    /// Get the width of the image
    pub fn width(&self) -> usize {
        self.width
    }

    /// Set the width of the image
    ///
    /// # Arguments
    /// * `width` - The width of the image.
    pub fn set_width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    /// Get the height of the image
    pub fn height(&self) -> usize {
        self.height
    }

    /// Set the height of the image
    ///
    /// # Arguments
    /// * `height` - The height of the image.
    pub fn set_height(mut self, height: usize) -> Self {
        self.height = height;
        self
    }

    /// Get the precision of the input image (bits per channel)
    pub fn precision(&self) -> u8 {
        self.precision
    }

    /// Set the precision of the input image (bits per channel)
    ///
    /// # Arguments
    /// * `precision` - The precision of the input image (bits per channel).
    pub fn set_precision(mut self, precision: u8) -> Self {
        self.precision = precision;
        self
    }

    /// Get the colorspace of the input image
    pub fn colorspace(&self) -> ColorSpace {
        self.colorspace
    }

    /// Set the colorspace of the input image
    ///
    /// # Arguments
    /// * `colorspace` - The colorspace of the input image.
    pub fn set_colorspace(mut self, colorspace: ColorSpace) -> Self {
        self.colorspace = colorspace;
        self
    }

    /// Get the chroma subsampling for the encoder
    pub fn chroma_subsampling(&self) -> (u8, u8) {
        self.chroma_subsampling
    }

    /// Set the chroma subsampling for the encoder
    ///
    /// # Arguments
    /// * `chroma_subsampling` - The chroma subsampling for the encoder (e.g. (4, 2) for 4:2:0).
    pub fn set_chroma_subsampling(mut self, chroma_subsampling: (u8, u8)) -> Self {
        self.chroma_subsampling = chroma_subsampling;
        self
    }

    /// Get the progressive encoding for the encoder
    pub fn progressive(&self) -> bool {
        self.progressive
    }

    /// Set the progressive encoding for the encoder
    ///
    /// # Arguments
    /// * `progressive` - Whether to use progressive encoding for the encoder.
    pub fn set_progressive(mut self, progressive: bool) -> Self {
        self.progressive = progressive;
        self
    }

    /// Get the lossless encoding for the encoder
    pub fn lossless(&self) -> bool {
        self.lossless
    }

    /// Set the lossless encoding for the encoder
    ///
    /// # Arguments
    /// * `lossless` - Whether to use lossless encoding for the encoder.
    pub fn set_lossless(mut self, lossless: bool) -> Self {
        self.lossless = lossless;
        self
    }

    /// Get the forced SIMD backend    ///
    /// If None, the encoder will choose the best available SIMD backend for the current platform.
    pub fn forced_simd_backend(&self) -> Option<SimdBackend> {
        self.forced_simd_backend
    }

    /// Set the forced SIMD backend
    ///
    /// # Arguments
    /// * `forced_simd_backend` - The SIMD backend to use for encoding.
    /// If None, the encoder will choose the best available SIMD backend for the current platform.
    pub fn set_forced_simd_backend(mut self, forced_simd_backend: Option<SimdBackend>) -> Self {
        self.forced_simd_backend = forced_simd_backend;
        self
    }
}
