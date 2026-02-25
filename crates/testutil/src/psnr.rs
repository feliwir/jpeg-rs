/// Compute the Peak Signal-to-Noise Ratio between two images.
pub fn compute_psnr<T>(original: &[T], decoded: &[T]) -> f64
where
    T: Into<f64> + Copy,
{
    assert_eq!(original.len(), decoded.len());
    let mse: f64 = original
        .iter()
        .zip(decoded.iter())
        .map(|(&a, &b)| {
            let diff = a.into() - b.into();
            diff * diff
        })
        .sum::<f64>()
        / original.len() as f64;

    if mse == 0.0 {
        return f64::INFINITY;
    }
    10.0 * (255.0 * 255.0 / mse).log10()
}
