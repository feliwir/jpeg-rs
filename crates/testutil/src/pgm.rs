use std::io::Read;

pub fn save_pixels_as_pgm(
    filename: &str,
    pixels: &[u8],
    width: usize,
    height: usize,
    bitdepth: usize,
) {
    use std::fs::File;
    use std::io::Write;
    let mut output = File::create(filename).unwrap();
    writeln!(output, "P5").unwrap();
    writeln!(output, "{} {} ", width, height).unwrap();
    writeln!(output, "{}", (1 << bitdepth) - 1).unwrap();

    if bitdepth > 8 {
        // PGM P5 with maxval > 255 requires big-endian 16-bit samples.
        // The decoder buffer stores little-endian u16, so swap bytes.
        let mut be_buf = Vec::with_capacity(pixels.len());
        for pair in pixels.chunks_exact(2) {
            be_buf.push(pair[1]);
            be_buf.push(pair[0]);
        }
        output.write_all(&be_buf).unwrap();
    } else {
        output.write_all(pixels).unwrap();
    }
}

pub fn load_pixels_from_pgm(data: &[u8]) -> (Vec<u8>, usize, usize, usize) {
    use std::io::BufRead;
    let mut reader = std::io::Cursor::new(data);
    let mut lines = reader.by_ref().lines();

    // Read magic number
    let magic = lines.next().unwrap().unwrap();
    assert_eq!(magic, "P5");

    // Read width, height
    let dims_line = lines.next().unwrap().unwrap();
    let mut dims_iter = dims_line.split_whitespace();
    let width: usize = dims_iter.next().unwrap().parse().unwrap();
    let height: usize = dims_iter.next().unwrap().parse().unwrap();

    // Read maxval
    let maxval_line = lines.next().unwrap().unwrap();
    let maxval: usize = maxval_line.parse().unwrap();
    assert!(maxval <= 65535);

    // Read pixel data
    let mut pixels = Vec::new();
    reader.read_to_end(&mut pixels).unwrap();

    (pixels, width, height, maxval)
}
