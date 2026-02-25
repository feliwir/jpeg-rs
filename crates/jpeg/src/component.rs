use crate::error::DecodeError;

pub(crate) const MAX_SAMPLING_FACTOR: usize = 4;
pub(crate) const MAX_COMPONENTS: usize = 4;

pub(crate) struct Component {
    /// The component id (1-based)
    pub id: u8,
    /// The horizontal sampling factor
    pub horizontal_sampling_factor: usize,
    /// The vertical sampling factor
    pub vertical_sampling_factor: usize,
    /// The quantization table id for this component
    pub quantization_table_id: usize,
    /// The AC huffman table id for this component
    pub ac_huffman_table_id: usize,
    /// The DC huffman table id for this component
    pub dc_huffman_table_id: usize,
}

impl Component {
    /// Create a new component with the given parameters.
    pub fn new(
        id: u8,
        horizontal_sampling_factor: usize,
        vertical_sampling_factor: usize,
        quantization_table_id: usize,
        dc_huffman_table_id: usize,
        ac_huffman_table_id: usize,
    ) -> Self {
        Component {
            id,
            horizontal_sampling_factor,
            vertical_sampling_factor,
            quantization_table_id,
            dc_huffman_table_id,
            ac_huffman_table_id,
        }
    }

    pub fn from_bytes(a: [u8; 3]) -> Result<Self, DecodeError> {
        let id = a[0];
        if id > 3 {
            return Err(DecodeError::Format(format!(
                "Unknown component id found: {id}"
            )));
        }

        let horizontal_sampling_factor = (a[1] >> 4) as usize;
        let vertical_sampling_factor = (a[1] & 0x0F) as usize;

        if horizontal_sampling_factor == 0 || horizontal_sampling_factor > MAX_SAMPLING_FACTOR {
            return Err(DecodeError::Format(format!(
                "Invalid horizontal sampling factor: {horizontal_sampling_factor}"
            )));
        }

        if vertical_sampling_factor == 0 || vertical_sampling_factor > MAX_SAMPLING_FACTOR {
            return Err(DecodeError::Format(format!(
                "Invalid vertical sampling factor: {vertical_sampling_factor}"
            )));
        }

        let quantization_table_id = a[2] as usize;
        if quantization_table_id >= MAX_COMPONENTS {
            return Err(DecodeError::Format(format!(
                "Invalid quantization table id: {quantization_table_id}"
            )));
        }

        Ok(Component {
            id,
            horizontal_sampling_factor,
            vertical_sampling_factor,
            quantization_table_id,
            ac_huffman_table_id: 0,
            dc_huffman_table_id: 0,
        })
    }
}
