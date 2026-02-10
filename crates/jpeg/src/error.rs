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
