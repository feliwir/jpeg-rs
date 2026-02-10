use crate::color_space::ColorSpace;

#[derive(Debug, Copy, Clone)]
pub enum SimdBackend {
    Scalar,
    #[allow(dead_code)]
    Neon,
    #[allow(dead_code)]
    Avx2,
}

impl SimdBackend {
    pub fn is_supported(&self) -> bool {
        match self {
            SimdBackend::Scalar => true,
            #[cfg(target_arch = "aarch64")]
            SimdBackend::Neon => true,
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdBackend::Avx2 => is_x86_feature_detected!("avx2"),
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
            backends.push(SimdBackend::Avx2);
        }
        backends.into_iter()
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
