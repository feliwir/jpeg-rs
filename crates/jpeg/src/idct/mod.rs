use jpeg_common::options::SimdBackend;

#[cfg(target_arch = "aarch64")]
pub mod neon;
pub mod scalar;
mod tables;

fn select_idct_internal<const PRECISION: u8>(
    forced_backend: Option<SimdBackend>,
) -> fn(&mut [i32; 64]) {
    if let Some(backend) = forced_backend {
        match backend {
            SimdBackend::Scalar => return scalar::idct::<PRECISION>,
            #[cfg(target_arch = "aarch64")]
            SimdBackend::Neon => return neon::idct::<PRECISION>,
            _ => return scalar::idct::<PRECISION>,
        }
    }

    // Check for NEON support at runtime, since some older ARMv8 CPUs may not have it.
    #[cfg(target_arch = "aarch64")]
    if std::arch::is_aarch64_feature_detected!("neon") {
        return neon::idct::<PRECISION>;
    }

    return scalar::idct::<PRECISION>;
}

pub(crate) fn select_idct_fn(
    precision: u8,
    forced_backend: Option<SimdBackend>,
) -> fn(&mut [i32; 64]) {
    // Check for NEON support at runtime, since some older ARMv8 CPUs may not have it.
    match precision {
        8 => select_idct_internal::<8>(forced_backend),
        10 => select_idct_internal::<10>(forced_backend),
        12 => select_idct_internal::<12>(forced_backend),
        _ => panic!("Unsupported precision: {}", precision),
    }
}
