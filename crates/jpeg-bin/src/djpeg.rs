//! djpeg — decompress a JPEG image to PGM or PPM.

use clap::Parser;
use jpeg::JpegDecoder;
use jpeg_common::color_space::ColorSpace;
use jpeg_common::options::{DecoderOptions, SimdBackend};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process;

/// Decompress a JPEG image to PGM (grayscale) or PPM (color).
#[derive(Parser)]
#[command(name = "djpeg", version, about)]
struct Cli {
    /// Input JPEG file (use "-" for stdin)
    input: PathBuf,

    /// Output PGM/PPM file (default: stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Maximum image width (reject larger images)
    #[arg(long, default_value_t = 16384)]
    max_width: usize,

    /// Maximum image height (reject larger images)
    #[arg(long, default_value_t = 16384)]
    max_height: usize,

    /// Output colorspace: grayscale, rgb, ycbcr
    #[arg(short = 'c', long)]
    colorspace: Option<String>,

    /// Force SIMD backend (scalar, sse, avx2, avx512, neon)
    #[arg(long)]
    simd: Option<String>,
}

fn parse_colorspace(s: &str) -> Result<ColorSpace, String> {
    match s.to_lowercase().as_str() {
        "grayscale" | "gray" | "grey" => Ok(ColorSpace::Grayscale),
        "rgb" => Ok(ColorSpace::RGB),
        "ycbcr" => Ok(ColorSpace::YCbCr),
        _ => Err(format!(
            "Unknown colorspace '{s}'. Use grayscale, rgb, or ycbcr"
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

/// Write pixels as PGM (P5) binary.
fn write_pgm<W: Write>(
    writer: &mut W,
    pixels: &[u8],
    width: usize,
    height: usize,
    precision: u8,
) -> std::io::Result<()> {
    let maxval = (1u32 << precision) - 1;
    writeln!(writer, "P5")?;
    writeln!(writer, "{width} {height}")?;
    writeln!(writer, "{maxval}")?;
    if precision > 8 {
        // PGM with maxval > 255 requires big-endian 16-bit samples.
        // Decoder stores little-endian u16, so swap bytes.
        let mut be_buf = Vec::with_capacity(pixels.len());
        for pair in pixels.chunks_exact(2) {
            be_buf.push(pair[1]);
            be_buf.push(pair[0]);
        }
        writer.write_all(&be_buf)?;
    } else {
        writer.write_all(pixels)?;
    }
    Ok(())
}

/// Write pixels as PPM (P6) binary.
fn write_ppm<W: Write>(
    writer: &mut W,
    pixels: &[u8],
    width: usize,
    height: usize,
) -> std::io::Result<()> {
    writeln!(writer, "P6")?;
    writeln!(writer, "{width} {height}")?;
    writeln!(writer, "255")?;
    writer.write_all(pixels)?;
    Ok(())
}

fn main() {
    let cli = Cli::parse();

    // Read input
    let input_data = fs::read(&cli.input).unwrap_or_else(|e| {
        eprintln!("Error reading '{}': {e}", cli.input.display());
        process::exit(1);
    });

    // Build decoder options
    let mut options = DecoderOptions::default()
        .set_max_width(cli.max_width)
        .set_max_height(cli.max_height);

    if let Some(ref cs) = cli.colorspace {
        let colorspace = parse_colorspace(cs).unwrap_or_else(|e| {
            eprintln!("Error: {e}");
            process::exit(1);
        });
        options = options.set_out_colorspace(Some(colorspace));
    }

    if let Some(ref s) = cli.simd {
        let backend = parse_simd_backend(s).unwrap_or_else(|e| {
            eprintln!("Error: {e}");
            process::exit(1);
        });
        options = options.set_forced_simd_backend(Some(backend));
    }

    // Decode
    let mut decoder = JpegDecoder::new_with_options(&input_data[..], options);
    let pixels = decoder.decode().unwrap_or_else(|e| {
        eprintln!("Error decoding JPEG: {e:?}");
        process::exit(1);
    });
    let info = decoder.info().unwrap_or_else(|| {
        eprintln!("Error: no image info available after decode");
        process::exit(1);
    });

    // Determine output format
    let is_grayscale = info.components == 1;

    // Build the output data
    let mut output_buf = Vec::new();
    if is_grayscale {
        write_pgm(
            &mut output_buf,
            &pixels,
            info.width,
            info.height,
            info.precision,
        )
        .unwrap();
    } else {
        write_ppm(&mut output_buf, &pixels, info.width, info.height).unwrap();
    }

    // Write output
    if let Some(ref path) = cli.output {
        fs::write(path, &output_buf).unwrap_or_else(|e| {
            eprintln!("Error writing '{}': {e}", path.display());
            process::exit(1);
        });
        eprintln!(
            "Decoded {}×{} {} image ({}-bit, {} component{})",
            info.width,
            info.height,
            if is_grayscale { "grayscale" } else { "color" },
            info.precision,
            info.components,
            if info.components == 1 { "" } else { "s" },
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
