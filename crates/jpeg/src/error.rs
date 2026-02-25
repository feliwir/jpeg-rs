#[derive(Debug)]
pub enum DecodeError {
    /// Any other thing we do not know
    Format(String),
    /// Any other thing we do not know but we
    /// don't need to allocate space on the heap
    FormatStatic(&'static str),
    /// Invalid marker found when expecting a specific one
    InvalidMarker(u16),
    /// problems with the Huffman Tables in a Decoder file
    HuffmanDecode(String),
    /// Image has zero width
    ZeroError,
    /// Discrete Quantization Tables error
    DqtError(String),
    /// Start of scan errors
    SosError(String),
    /// Start of frame errors
    SofError(String),
    /// MCU errors
    MCUError(String),
    /// Exhausted data
    ExhaustedData,
    /// Large image dimensions(Corrupted data)?
    LargeDimensions(usize),
    /// Too small output for size
    TooSmallOutput(usize, usize),
    /// Unsupported features
    Unsupported(String),
    /// IO errors
    IoErrors(std::io::Error),
}

impl From<std::io::Error> for DecodeError {
    fn from(err: std::io::Error) -> Self {
        DecodeError::IoErrors(err)
    }
}

#[derive(Debug)]
pub enum EncodeError {
    /// Invalid dimensions
    InvalidDimensions(String),
    /// Invalid or unsupported color space
    UnsupportedColorSpace,
    /// Invalid options
    InvalidOptions(String),
    /// Unsupported feature
    Unsupported(String),
    /// IO errors
    IoErrors(std::io::Error),
}

impl From<std::io::Error> for EncodeError {
    fn from(err: std::io::Error) -> Self {
        EncodeError::IoErrors(err)
    }
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::InvalidDimensions(msg) => write!(f, "Invalid dimensions: {}", msg),
            EncodeError::UnsupportedColorSpace => write!(f, "Unsupported color space"),
            EncodeError::InvalidOptions(msg) => write!(f, "Invalid options: {}", msg),
            EncodeError::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
            EncodeError::IoErrors(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for EncodeError {}
