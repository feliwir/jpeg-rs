//! cjpeg — compress a PGM or PPM image to JPEG.

use clap::Parser;
use jpeg::JpegEncoder;
use jpeg_common::color_space::ColorSpace;
use jpeg_common::options::{EncoderOptions, SimdBackend};
use std::fs;
use std::io::{BufRead, Cursor, Read, Write};
use std::path::PathBuf;
use std::process;

/// Compress a PGM/PPM image to JPEG.
#[derive(Parser)]
#[command(name = "cjpeg", version, about)]
struct Cli {
    /// Input PGM or PPM file (use "-" for stdin)
    input: PathBuf,

    /// Output JPEG file (default: stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Compression quality (1–100)
    #[arg(short, long, default_value_t = 85, value_parser = clap::value_parser!(u8).range(1..=100))]
    quality: u8,

    /// Chroma subsampling: "4:4:4", "4:2:2", or "4:2:0"
    #[arg(short = 's', long, default_value = "4:2:0")]
    subsampling: String,

    /// Force SIMD backend (scalar, sse, avx2, avx512, neon)
    #[arg(long)]
    simd: Option<String>,

    /// Use progressive encoding
    #[arg(long)]
    progressive: bool,

    /// Use lossless encoding
    #[arg(long)]
    lossless: bool,
}

fn parse_subsampling(s: &str) -> Result<(u8, u8), String> {
    match s {
        "4:4:4" | "444" => Ok((4, 4)),
        "4:2:2" | "422" => Ok((4, 1)),
        "4:2:0" | "420" => Ok((4, 2)),
        _ => Err(format!(
            "Unknown subsampling '{s}'. Use 4:4:4, 4:2:2, or 4:2:0"
        )),
    }
}

fn parse_simd_backend(s: &str) -> Result<SimdBackend, String> {
    match s.to_lowercase().as_str() {
        "scalar" => Ok(SimdBackend::Scalar),
        "sse" => Ok(SimdBackend::Sse),
        "avx2" => Ok(SimdBackend::Avx2),
        "avx512" => Ok(SimdBackend::Avx512),
        "neon" => Ok(SimdBackend::Neon),
        _ => Err(format!(
            "Unknown SIMD backend '{s}'. Use scalar, sse, avx2, avx512, or neon"
        )),
    }
}

/// Load a PGM (P5) or PPM (P6) file.
///
/// Returns (pixels, width, height, components).
fn load_image(data: &[u8]) -> Result<(Vec<u8>, usize, usize, usize), String> {
    let mut reader = Cursor::new(data);
    let mut header_lines: Vec<String> = Vec::new();

    // Read header lines, skipping comments
    while header_lines.len() < 3 {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|e| format!("Failed to read header: {e}"))?;
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        header_lines.push(trimmed);
    }

    let magic = &header_lines[0];
    let components = match magic.as_str() {
        "P5" => 1,
        "P6" => 3,
        _ => {
            return Err(format!(
                "Unsupported format '{magic}'. Only P5 (PGM) and P6 (PPM) are supported"
            ));
        }
    };

    let mut dims = header_lines[1].split_whitespace();
    let width: usize = dims
        .next()
        .ok_or("Missing width")?
        .parse()
        .map_err(|e| format!("Invalid width: {e}"))?;
    let height: usize = dims
        .next()
        .ok_or("Missing height")?
        .parse()
        .map_err(|e| format!("Invalid height: {e}"))?;

    let _maxval: usize = header_lines[2]
        .parse()
        .map_err(|e| format!("Invalid maxval: {e}"))?;

    let mut pixels = Vec::new();
    reader
        .read_to_end(&mut pixels)
        .map_err(|e| format!("Failed to read pixel data: {e}"))?;

    let expected = width * height * components;
    if pixels.len() < expected {
        return Err(format!(
            "Pixel data too short: expected {expected} bytes, got {}",
            pixels.len()
        ));
    }
    pixels.truncate(expected);

    Ok((pixels, width, height, components))
}

fn main() {
    let cli = Cli::parse();

    // Read input
    let input_data = fs::read(&cli.input).unwrap_or_else(|e| {
        eprintln!("Error reading '{}': {e}", cli.input.display());
        process::exit(1);
    });

    // Parse the image
    let (pixels, width, height, components) = load_image(&input_data).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        process::exit(1);
    });

    let colorspace = if components == 1 {
        ColorSpace::Grayscale
    } else {
        ColorSpace::RGB
    };

    let subsampling = parse_subsampling(&cli.subsampling).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        process::exit(1);
    });

    let simd_backend = cli.simd.as_deref().map(|s| {
        parse_simd_backend(s).unwrap_or_else(|e| {
            eprintln!("Error: {e}");
            process::exit(1);
        })
    });

    // Build encoder options
    let options = EncoderOptions::new(width, height, colorspace)
        .set_quality(cli.quality)
        .set_chroma_subsampling(subsampling)
        .set_progressive(cli.progressive)
        .set_lossless(cli.lossless)
        .set_forced_simd_backend(simd_backend);

    // Encode
    let mut output_buf = Vec::new();
    let mut encoder = JpegEncoder::new_with_options(&mut output_buf, options).unwrap_or_else(|e| {
        eprintln!("Error creating encoder: {e}");
        process::exit(1);
    });
    encoder.encode(&pixels).unwrap_or_else(|e| {
        eprintln!("Error encoding JPEG: {e}");
        process::exit(1);
    });

    // Write output
    if let Some(ref path) = cli.output {
        fs::write(path, &output_buf).unwrap_or_else(|e| {
            eprintln!("Error writing '{}': {e}", path.display());
            process::exit(1);
        });
        eprintln!(
            "Encoded {}×{} {} image → {} bytes (quality {})",
            width,
            height,
            if components == 1 {
                "grayscale"
            } else {
                "color"
            },
            output_buf.len(),
            cli.quality,
        );
    } else {
        std::io::stdout()
            .write_all(&output_buf)
            .unwrap_or_else(|e| {
                eprintln!("Error writing to stdout: {e}");
                process::exit(1);
            });
    }
}
