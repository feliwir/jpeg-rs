use jpeg_common::color_space::ColorSpace;

use super::Marker;
use crate::{
    JpegDecoder,
    component::{Component, MAX_COMPONENTS},
    constants::UN_ZIGZAG,
    error::DecodeError,
    huffman::HuffmanTable,
    idct,
};

/// Section:`B.2.2 Frame header syntax`
pub(crate) fn read_start_of_frame<R: std::io::BufRead>(
    decoder: &mut JpegDecoder<R>,
    _marker: Marker,
) -> Result<(), DecodeError> {
    if decoder.did_read_sof {
        return Err(DecodeError::SofError(
            "Multiple SOF segments found, only one is allowed".to_string(),
        ));
    }
    decoder.did_read_sof = true;

    let length = decoder.read_length()?.checked_sub(2).ok_or_else(|| {
        DecodeError::SofError("Invalid SOF length, must be greater than 2".to_string())
    })?;
    let mut buf = vec![0u8; length];
    decoder.reader.read_exact(&mut buf)?;

    // Parse the SOF segment to extract image information
    // For simplicity, we will just parse the width and height here
    if buf.len() < 6 {
        return Err(DecodeError::SofError(format!(
            "SOF segment too short: expected at least 6 bytes, got {}",
            buf.len()
        )));
    }

    let precision = buf[0];
    let height = u16::from_be_bytes([buf[1], buf[2]]) as usize;
    let width = u16::from_be_bytes([buf[3], buf[4]]) as usize;

    log::trace!(
        "Image dimensions: {}x{}, precision: {}",
        width,
        height,
        precision
    );

    decoder.info.width = width;
    decoder.info.height = height;
    decoder.info.precision = precision;

    decoder.info.components = buf[5] as usize;
    if decoder.info.components == 0 {
        return Err(DecodeError::SofError(
            "SOF segment must have at least one component".to_string(),
        ));
    }

    let expected = 6 + 3 * decoder.info.components;
    // length should be equal to num components
    if length != expected {
        return Err(DecodeError::SofError(format!(
            "Length of start of frame differs from expected {expected},value is {length}"
        )));
    }

    match decoder.info.components {
        1 => decoder.input_colorspace = ColorSpace::Grayscale,
        3 => decoder.input_colorspace = ColorSpace::YCbCr,
        4 => decoder.input_colorspace = ColorSpace::CMYK,
        _ => {
            return Err(DecodeError::SofError(format!(
                "Unsupported number of components: {}",
                decoder.info.components
            )));
        }
    }

    // Read 3 bytes for each component (component id, horizontal sampling factor, vertical sampling factor)
    decoder.components.reserve_exact(decoder.info.components);
    for i in 0..decoder.info.components {
        let component_data = [buf[6 + 3 * i], buf[6 + 3 * i + 1], buf[6 + 3 * i + 2]];
        log::debug!(
            "SOF component {}: id={}, hsamp={}, vsamp={}, qtable={}",
            i,
            component_data[0],
            component_data[1] >> 4,
            component_data[1] & 0x0F,
            component_data[2]
        );
        let component = Component::from_bytes(component_data)?;
        decoder.components.push(component);
    }

    Ok(())
}

pub(crate) fn read_start_of_scan<R: std::io::BufRead>(
    decoder: &mut JpegDecoder<R>,
) -> Result<(), DecodeError> {
    let length = decoder.read_length()?.checked_sub(2).ok_or_else(|| {
        DecodeError::SofError("Invalid SOS length, must be greater than 2".to_string())
    })?;
    let mut buf = vec![0u8; length];
    decoder.reader.read_exact(&mut buf)?;

    let ns = buf[0] as usize; // Number of components in scan

    // Check number of components.
    if !(1..5).contains(&ns) {
        return Err(DecodeError::SosError(format!(
            "Invalid number of components in start of scan {ns}, expected in range 1..5"
        )));
    }

    for i in 0..ns {
        let component_id = buf[1 + 2 * i];
        let dc_ac = buf[1 + 2 * i + 1];
        let dc_table_id = dc_ac >> 4;
        let ac_table_id = dc_ac & 0x0F;

        log::debug!(
            "SOS component {}: id={}, dc_table={}, ac_table={}",
            i,
            component_id,
            dc_table_id,
            ac_table_id
        );

        // if component_id == 0 {
        //     return Err(DecodeError::SosError(
        //         "Invalid component id in start of scan: 0".to_string(),
        //     ));
        // }

        if dc_table_id > 3 {
            return Err(DecodeError::SosError(format!(
                "Invalid DC huffman table id: {dc_table_id}"
            )));
        }

        if ac_table_id > 3 {
            return Err(DecodeError::SosError(format!(
                "Invalid AC huffman table id: {ac_table_id}"
            )));
        }

        let component = decoder
            .components
            .iter_mut()
            .find(|c| c.id == component_id)
            .ok_or_else(|| {
                DecodeError::SosError(format!(
                    "Component with id {component_id} not found in start of scan"
                ))
            })?;
        component.ac_huffman_table_id = ac_table_id as usize;
        component.dc_huffman_table_id = dc_table_id as usize;
        decoder.z_order[i] = component_id as usize;

        log::trace!(
            "Component {} uses DC huffman table {} and AC huffman table {}",
            component_id,
            dc_table_id,
            ac_table_id
        );
    }

    decoder.num_scans = ns;

    Ok(())
}

/// Small utility function to print Un-zig-zagged quantization tables
fn un_zig_zag<T>(a: &[T]) -> [i32; 64]
where
    T: Default + Copy,
    i32: core::convert::From<T>,
{
    let mut output = [i32::default(); 64];

    for i in 0..64 {
        output[UN_ZIGZAG[i]] = i32::from(a[i]);
    }

    output
}

///**B.2.4.1 Quantization table-specification syntax**
pub(crate) fn read_quant_tables<R: std::io::BufRead>(
    decoder: &mut JpegDecoder<R>,
) -> Result<(), DecodeError> {
    let length = decoder
        .read_length()?
        .checked_sub(2)
        .ok_or(DecodeError::FormatStatic(
            "Invalid DQT length, must be greater than 2",
        ))?;
    let mut buf = vec![0u8; length];
    decoder.reader.read_exact(&mut buf)?;

    let mut offset = 0;
    while offset < buf.len() {
        let qt_info = buf[offset];
        offset += 1;

        // 0 = 8 bit otherwise 16 bit dqt
        let precision = qt_info >> 4;
        let table_id = (qt_info & 0x0F) as usize;

        if table_id >= MAX_COMPONENTS {
            return Err(DecodeError::DqtError(format!(
                "Invalid quantization table id: {table_id}"
            )));
        }

        let table_size = if precision == 0 { 64 } else { 128 };
        log::trace!(
            "Reading quantization table with id {table_id}, precision: {} bits",
            if precision == 0 { 8 } else { 16 }
        );

        if offset + table_size > buf.len() {
            return Err(DecodeError::DqtError(format!(
                "Quantization table data is too short: expected {} bytes, got {}",
                table_size,
                buf.len() - offset
            )));
        }

        // We will un-zigzag the quantization tables here, so we can use them directly when decoding the image data
        decoder.quantization_tables[table_id as usize] =
            Some(un_zig_zag(&buf[offset..offset + table_size]));
        offset += table_size;
    }

    Ok(())
}

///**B.2.4.2 Huffman table-specification syntax**
pub(crate) fn read_huffman_tables<R: std::io::BufRead>(
    decoder: &mut JpegDecoder<R>,
) -> Result<(), DecodeError> {
    let length = decoder
        .read_length()?
        .checked_sub(2)
        .ok_or(DecodeError::FormatStatic(
            "Invalid DHT length, must be greater than 2",
        ))?;

    let mut buf = vec![0u8; length];
    decoder.reader.read_exact(&mut buf)?;

    let mut offset = 0;
    while (offset + 17) < buf.len() {
        let table_info = buf[offset];
        let table_class = table_info >> 4; // 0 = DC, 1 = AC
        let table_id = table_info & 0x0F;
        offset += 1;

        if table_id >= MAX_COMPONENTS as u8 {
            return Err(DecodeError::Format(format!(
                "Invalid Huffman table id: {table_id}"
            )));
        }

        if table_class > 1 {
            return Err(DecodeError::Format(format!(
                "Invalid Huffman table class: {table_class}"
            )));
        }

        // Read the number of symbols for each code length (1 to 16)
        let mut num_symbols: [u8; 17] = [0; 17];
        num_symbols[1..17].copy_from_slice(&buf[offset..offset + 16]);

        let symbols_sum = num_symbols.iter().map(|&x| x as usize).sum::<usize>();
        offset += 16;

        // If the sum is greater than 256, it's invalid
        if symbols_sum > 256 {
            return Err(DecodeError::Format(format!(
                "Invalid Huffman table: total symbols exceed 256 (got {})",
                symbols_sum
            )));
        }

        offset += symbols_sum;
        if offset > buf.len() {
            return Err(DecodeError::Format(format!(
                "Huffman table data is too short: expected {} bytes, got {}",
                offset,
                buf.len()
            )));
        }

        let mut symbols: [u8; 256] = [0; 256];
        symbols[..symbols_sum].copy_from_slice(&buf[offset - symbols_sum..offset]);

        match table_class {
            // DC Table
            0 => {
                decoder.dc_huffman_tables[table_id as usize] =
                    Some(HuffmanTable::new(&num_symbols, symbols)?)
            }
            // AC Table
            1 => {
                decoder.ac_huffman_tables[table_id as usize] =
                    Some(HuffmanTable::new(&num_symbols, symbols)?)
            }
            _ => unreachable!(),
        }
    }

    // Check if we have read all the data in the DHT segment
    if offset != buf.len() {
        return Err(DecodeError::Format(format!(
            "Extra data found in DHT segment: expected {} bytes, got {}",
            offset,
            buf.len()
        )));
    }

    Ok(())
}
