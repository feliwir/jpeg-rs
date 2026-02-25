//! JPEG marker writing utilities

use super::Marker;
use crate::component::Component;
use crate::constants::UN_ZIGZAG;
use crate::error::EncodeError;
use std::io::Write;

/// Write a marker with no payload to the writer
pub(crate) fn write_marker<W: Write>(writer: &mut W, marker: Marker) -> Result<(), EncodeError> {
    if let Some(byte) = marker.into_u8() {
        writer.write_all(&[0xFF, byte])?;
        Ok(())
    } else {
        Err(EncodeError::Unsupported(
            "Cannot write marker type".to_string(),
        ))
    }
}

/// Write APP0 (JFIF) marker
pub(crate) fn write_app0<W: Write>(writer: &mut W) -> Result<(), EncodeError> {
    let app0 = [
        0xFF, 0xE0, // APP0 marker
        0x00, 0x10, // Length (16 bytes)
        0x4A, 0x46, 0x49, 0x46, 0x00, // "JFIF\0"
        0x01, 0x01, // Version 1.1
        0x00, // Units (0 = no units)
        0x00, 0x01, // X density
        0x00, 0x01, // Y density
        0x00, 0x00, // Thumbnail width and height
    ];
    writer.write_all(&app0)?;
    Ok(())
}

/// Write DQT (Define Quantization Table) markers
pub(crate) fn write_dqt<W: Write>(writer: &mut W, quality: u8) -> Result<(), EncodeError> {
    let q = if quality < 50 {
        (5000 / quality as i32) as u8
    } else {
        (200 - 2 * quality as i32) as u8
    }
    .max(1);

    // DQT marker for luminance (Y)
    let mut dqt_data = vec![0xFF, 0xDB];
    dqt_data.push(0x00);
    dqt_data.push(67); // Length (2 + 1 + 64)
    dqt_data.push(0x00); // Precision=0, Table class=0 (luminance)

    // Standard JPEG quantization table for Y (row-major order)
    #[rustfmt::skip]
    let base_table: [u8; 64] = [
        16, 11, 10, 16, 24, 40, 51, 61,
        12, 12, 14, 19, 26, 58, 60, 55,
        14, 13, 16, 24, 40, 57, 69, 56,
        14, 17, 22, 29, 51, 87, 80, 62,
        18, 22, 37, 56, 68,109,103, 77,
        24, 35, 55, 64, 81,104,113, 92,
        49, 64, 78, 87,103,121,120,101,
        72, 92, 95, 98,112,100,103, 99,
    ];

    // DQT values must be stored in zigzag scan order (ITU-T T.81 §B.2.4.1)
    for k in 0..64 {
        let val = base_table[UN_ZIGZAG[k]];
        let scaled = ((val as u32 * q as u32) / 100).min(255) as u8;
        dqt_data.push(scaled);
    }

    writer.write_all(&dqt_data)?;

    // DQT marker for chrominance (Cb, Cr)
    let mut dqt_data = vec![0xFF, 0xDB];
    dqt_data.push(0x00);
    dqt_data.push(67); // Length
    dqt_data.push(0x01); // Precision=0, Table class=1 (chrominance)

    // Standard JPEG quantization table for Cb/Cr (row-major order)
    #[rustfmt::skip]
    let chroma_table: [u8; 64] = [
        17, 18, 24, 47, 99, 99, 99, 99,
        18, 21, 26, 66, 99, 99, 99, 99,
        24, 26, 56, 99, 99, 99, 99, 99,
        47, 66, 99, 99, 99, 99, 99, 99,
        99, 99, 99, 99, 99, 99, 99, 99,
        99, 99, 99, 99, 99, 99, 99, 99,
        99, 99, 99, 99, 99, 99, 99, 99,
        99, 99, 99, 99, 99, 99, 99, 99,
    ];

    // DQT values must be stored in zigzag scan order (ITU-T T.81 §B.2.4.1)
    for k in 0..64 {
        let val = chroma_table[UN_ZIGZAG[k]];
        let scaled = ((val as u32 * q as u32) / 100).min(255) as u8;
        dqt_data.push(scaled);
    }

    writer.write_all(&dqt_data)?;
    Ok(())
}

/// Write a SOF0 (Start of Frame – Baseline) marker.
///
/// `components` describes each image component (id, sampling factors,
/// quantization table selector).
pub(crate) fn write_sof0<W: Write>(
    writer: &mut W,
    width: u16,
    height: u16,
    precision: u8,
    components: &[Component],
) -> Result<(), EncodeError> {
    // Length = 2 (length field) + 1 (precision) + 2 (height) + 2 (width)
    //        + 1 (num components) + 3 * num_components
    let length = (8 + 3 * components.len()) as u16;

    let mut sof = vec![0xFF, 0xC0]; // SOF0 marker
    sof.push((length >> 8) as u8);
    sof.push((length & 0xFF) as u8);
    sof.push(precision);

    // Height and width (big-endian)
    sof.push((height >> 8) as u8);
    sof.push((height & 0xFF) as u8);
    sof.push((width >> 8) as u8);
    sof.push((width & 0xFF) as u8);

    sof.push(components.len() as u8);

    for comp in components {
        sof.push(comp.id);
        sof.push(
            ((comp.horizontal_sampling_factor as u8) << 4) | comp.vertical_sampling_factor as u8,
        );
        sof.push(comp.quantization_table_id as u8);
    }

    writer.write_all(&sof)?;
    Ok(())
}

/// Write DHT (Define Huffman Table) markers with standard tables.
///
/// For color images (`num_components > 1`), writes both luminance and
/// chrominance tables.  For grayscale, only luminance tables are needed.
pub(crate) fn write_dht<W: Write>(
    writer: &mut W,
    num_components: usize,
) -> Result<(), EncodeError> {
    use crate::huffman_encode::*;

    // DC table 0 (luminance)
    write_dht_table(writer, 0x00, &DC_LUM_LENGTHS, DC_LUM_VALUES)?;
    // AC table 0 (luminance)
    write_dht_table(writer, 0x10, &AC_LUM_LENGTHS, AC_LUM_VALUES)?;

    if num_components > 1 {
        // DC table 1 (chrominance)
        write_dht_table(writer, 0x01, &DC_CHROM_LENGTHS, DC_CHROM_VALUES)?;
        // AC table 1 (chrominance)
        write_dht_table(writer, 0x11, &AC_CHROM_LENGTHS, AC_CHROM_VALUES)?;
    }

    Ok(())
}

/// Write a single DHT table
fn write_dht_table<W: Write>(
    writer: &mut W,
    table_info: u8,
    lengths: &[u8; 16],
    values: &[u8],
) -> Result<(), EncodeError> {
    let mut dht = vec![0xFF, 0xC4]; // DHT marker

    let length = (2 + 1 + 16 + values.len()) as u16;
    dht.push((length >> 8) as u8);
    dht.push((length & 0xFF) as u8);

    dht.push(table_info); // Table class and destination

    for &len in lengths {
        dht.push(len);
    }

    dht.extend_from_slice(values);

    writer.write_all(&dht)?;
    Ok(())
}

/// Write a SOS (Start of Scan) marker.
///
/// `components` lists each scan component with its Huffman table selectors.
pub(crate) fn write_sos<W: Write>(
    writer: &mut W,
    components: &[Component],
) -> Result<(), EncodeError> {
    // Length = 2 (length field) + 1 (Ns) + 2*Ns + 3 (Ss, Se, Ah|Al)
    let length = (6 + 2 * components.len()) as u16;

    let mut sos = vec![0xFF, 0xDA]; // SOS marker
    sos.push((length >> 8) as u8);
    sos.push((length & 0xFF) as u8);
    sos.push(components.len() as u8);

    for comp in components {
        sos.push(comp.id);
        sos.push(((comp.dc_huffman_table_id as u8) << 4) | comp.ac_huffman_table_id as u8);
    }

    // Spectral selection start, end, and successive approximation
    sos.push(0x00); // Ss
    sos.push(0x3F); // Se
    sos.push(0x00); // Ah | Al

    writer.write_all(&sos)?;
    Ok(())
}
