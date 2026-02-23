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
    output.write_all(pixels).unwrap();
}
