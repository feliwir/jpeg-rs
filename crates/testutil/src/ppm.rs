use std::io::Read;

pub fn save_pixels_as_ppm(filename: &str, pixels: &[u8], width: usize, height: usize) {
    use std::fs::File;
    use std::io::Write;
    if let Some(parent) = std::path::Path::new(filename).parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let mut output = File::create(filename).unwrap();
    writeln!(output, "P6").unwrap();
    writeln!(output, "{} {} ", width, height).unwrap();
    writeln!(output, "255").unwrap();
    output.write_all(pixels).unwrap();
}

pub fn load_pixels_from_ppm(data: &[u8]) -> (Vec<u8>, usize, usize) {
    use std::io::BufRead;
    let mut reader = std::io::Cursor::new(data);
    let mut lines = reader.by_ref().lines();

    // Read magic number
    let magic = lines.next().unwrap().unwrap();
    assert_eq!(magic, "P6");

    // Read width, height
    let dims_line = lines.next().unwrap().unwrap();
    let mut dims_iter = dims_line.split_whitespace();
    let width: usize = dims_iter.next().unwrap().parse().unwrap();
    let height: usize = dims_iter.next().unwrap().parse().unwrap();

    // Read maxval
    let maxval_line = lines.next().unwrap().unwrap();
    let maxval: usize = maxval_line.parse().unwrap();
    assert_eq!(maxval, 255);

    // Read pixel data
    let mut pixels = Vec::new();
    reader.read_to_end(&mut pixels).unwrap();

    (pixels, width, height)
}
