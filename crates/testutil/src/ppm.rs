pub fn save_pixels_as_ppm(filename: &str, pixels: &[u8], width: usize, height: usize) {
    use std::fs::File;
    use std::io::Write;
    let mut output = File::create(filename).unwrap();
    writeln!(output, "P6").unwrap();
    writeln!(output, "{} {} ", width, height).unwrap();
    writeln!(output, "255").unwrap();
    output.write_all(pixels).unwrap();
}
