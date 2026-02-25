pub mod scalar;
mod tables;

/// Function pointer type for the forward DCT.
///
/// Transforms an 8×8 block of level-shifted pixel values (in-place) to
/// DCT coefficients in row-major order.
///
/// # Safety
/// SIMD variants require the corresponding CPU features to be available.
pub type DctFn = unsafe fn(&mut [i32; 64]);

/// Select the best available forward DCT function for the current
/// platform, optionally forced to a specific SIMD backend.
pub(crate) fn select_dct_fn(forced_backend: Option<jpeg_common::options::SimdBackend>) -> DctFn {
    if let Some(backend) = forced_backend {
        match backend {
            jpeg_common::options::SimdBackend::Scalar => return scalar::fdct_fixed,
            // TODO: add SIMD backends
            _ => return scalar::fdct_fixed,
        }
    }

    // Default: fixed-point scalar
    scalar::fdct_fixed
}
