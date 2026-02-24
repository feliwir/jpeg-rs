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
