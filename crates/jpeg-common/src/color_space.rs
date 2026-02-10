#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    Unknown,
    Grayscale,
    RGB,
    YCbCr,
    CMYK,
}

impl ColorSpace {
    pub fn num_components(&self) -> usize {
        match self {
            ColorSpace::Unknown => 0,
            ColorSpace::Grayscale => 1,
            ColorSpace::RGB => 3,
            ColorSpace::YCbCr => 3,
            ColorSpace::CMYK => 4,
        }
    }
}
