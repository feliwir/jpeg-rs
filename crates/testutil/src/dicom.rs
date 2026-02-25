use std::borrow::Cow;

use dicom::{core::DicomValue, dictionary_std::tags, object::FileDicomObject};

pub fn extract_pixel_data_from_dicom(dicom_data: &[u8], frame_number: usize) -> Vec<u8> {
    let dcm = FileDicomObject::from_reader(dicom_data).expect("Failed to parse DICOM data");

    let pixeldata = dcm
        .element(tags::PIXEL_DATA)
        .expect("Missing PixelData element");
    match pixeldata.value() {
        DicomValue::PixelSequence(seq) => {
            let number_of_frames = match dcm.get(tags::NUMBER_OF_FRAMES) {
                Some(elem) => elem.to_int::<u32>().unwrap_or_else(|e| {
                    panic!("Invalid Number of Frames: {}", e);
                    1
                }),
                None => 1,
            };

            if number_of_frames as usize == seq.fragments().len() {
                // frame-to-fragment mapping is 1:1

                // get fragment containing our frame
                let fragment = seq
                    .fragments()
                    .get(frame_number as usize)
                    .expect("Frame number exceeds available fragments");

                fragment.to_vec()
            } else {
                // In this case we look up the basic offset table
                // and gather all of the frame's fragments in a single vector.
                // Note: not the most efficient way to do this,
                // consider optimizing later with byte chunk readers
                let offset_table = seq.offset_table();
                let base_offset = offset_table.get(frame_number as usize).copied();
                let base_offset = if frame_number == 0 {
                    base_offset.unwrap_or(0) as usize
                } else {
                    base_offset.expect("Missing offset entry for frame") as usize
                };
                let next_offset = offset_table.get(frame_number as usize + 1);

                let mut offset = 0;
                let mut frame_data = Vec::new();
                for fragment in seq.fragments() {
                    // include it
                    if offset >= base_offset {
                        frame_data.extend_from_slice(fragment);
                    }
                    offset += fragment.len() + 8;
                    if let Some(&next_offset) = next_offset {
                        if offset >= next_offset as usize {
                            // next fragment is for the next frame
                            break;
                        }
                    }
                }

                frame_data
            }
        }
        DicomValue::Primitive(v) => {
            // grab the intended slice based on image properties

            let get_int_property = |tag, _name| {
                dcm.get(tag)
                    .expect("Missing property")
                    .to_int::<usize>()
                    .expect("Invalid property value")
            };

            let rows = get_int_property(tags::ROWS, "Rows");
            let columns = get_int_property(tags::COLUMNS, "Columns");
            let samples_per_pixel = get_int_property(tags::SAMPLES_PER_PIXEL, "Samples Per Pixel");
            let bits_allocated = get_int_property(tags::BITS_ALLOCATED, "Bits Allocated");
            let frame_size = rows * columns * samples_per_pixel * ((bits_allocated + 7) / 8);

            let frame = frame_number as usize;
            let mut data = v.to_bytes();
            match &mut data {
                Cow::Borrowed(data) => {
                    *data = data
                        .get((frame_size * frame)..(frame_size * (frame + 1)))
                        .expect("Frame number exceeds available fragments");
                }
                Cow::Owned(data) => {
                    *data = data
                        .get((frame_size * frame)..(frame_size * (frame + 1)))
                        .expect("Frame number exceeds available fragments")
                        .to_vec();
                }
            }
            data.to_vec()
        }
        _ => {
            panic!("Unsupported PixelData format: expected PixelSequence or Primitive")
        }
    }
}
