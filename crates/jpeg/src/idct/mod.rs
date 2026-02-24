use jpeg_common::options::SimdBackend;
#[cfg(any(target_arch = "x86_64"))]
pub mod avx512;
#[cfg(target_arch = "aarch64")]
pub mod neon;
pub mod scalar;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod sse;
mod tables;

/// Function pointer type for IDCT.
///
/// Converts an 8x8 block of frequency coefficients (in-place) to spatial domain pixel values,
/// with the DC coefficient at index 0 and AC coefficients in zig-zag order.
///
/// # Safety
/// SIMD variants require the corresponding CPU features to be available.
pub type IdctFn = unsafe fn(&mut [i32; 64]);

/// Select the best available IDCT function for the current
/// platform, optionally forced to a specific SIMD backend.
fn select_idct_internal<const PRECISION: u8>(forced_backend: Option<SimdBackend>) -> IdctFn {
    if let Some(backend) = forced_backend {
        match backend {
            SimdBackend::Scalar => return scalar::idct_fixed::<PRECISION>,
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdBackend::Avx512 => return avx512::idct_fixed::<PRECISION>,
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdBackend::Avx2 | SimdBackend::Sse => return sse::idct_fixed::<PRECISION>,
            #[cfg(target_arch = "aarch64")]
            SimdBackend::Neon => return neon::idct::<PRECISION>,
            _ => return scalar::idct::<PRECISION>,
        }
    }

    // Check for NEON support at runtime, since some older ARMv8 CPUs may not have it.
    #[cfg(target_arch = "aarch64")]
    if SimdBackend::is_supported(SimdBackend::Neon) {
        return neon::idct::<PRECISION>;
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if SimdBackend::is_supported(SimdBackend::Avx512) {
        return avx512::idct_fixed::<PRECISION>;
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if SimdBackend::is_supported(SimdBackend::Sse) {
        return sse::idct_fixed::<PRECISION>;
    }

    return scalar::idct::<PRECISION>;
}

pub(crate) fn select_idct_fn(
    precision: u8,
    forced_backend: Option<SimdBackend>,
) -> unsafe fn(&mut [i32; 64]) {
    // Check for NEON support at runtime, since some older ARMv8 CPUs may not have it.
    match precision {
        8 => select_idct_internal::<8>(forced_backend),
        12 => select_idct_internal::<12>(forced_backend),
        _ => panic!("Unsupported precision: {}", precision),
    }
}
