use std::f32::consts::FRAC_1_SQRT_2;

/// Fixed-point scaling factor (must match the DCT table precision).
pub const FIX_BITS: i32 = 14;

/// Precomputed cosine values for the forward DCT.
///
/// Uses integer arithmetic with a fixed-point representation (scaled by 2^FIX_BITS).
/// Formula: cos((2x+1)·u·π/16) pre-multiplied by cu normalization factor (1/√2 for u=0, 1 otherwise).
///
/// Note: The DCT and IDCT share the same cosine kernel — the only difference
/// is the direction of the summation (over spatial vs frequency indices).
/// We reuse the same table as the IDCT.
#[rustfmt::skip]
pub const COS_TABLE_FIXED: [[i32; 8]; 8] = [
    [11585, 16069, 15137, 13623, 11585, 9102, 6270, 3196],
    [11585, 13623, 6270, -3196, -11585, -16069, -15137, -9102],
    [11585, 9102, -6270, -16069, -11585, 3196, 15137, 13623],
    [11585, 3196, -15137, -9102, 11585, 13623, -6270, -16069],
    [11585, -3196, -15137, 9102, 11585, -13623, -6270, 16069],
    [11585, -9102, -6270, 16069, -11585, -3196, 15137, -13623],
    [11585, -13623, 6270, 3196, -11585, 16069, -15137, 9102],
    [11585, -16069, 15137, -13623, 11585, -9102, 6270, -3196],
];

/// Precomputed cosine values for the forward DCT (floating point).
///
/// Formula: cos((2x+1)·u·π/16) pre-multiplied by cu normalization factor.
#[rustfmt::skip]
pub const COS_TABLE: [[f32; 8]; 8] = [
    [FRAC_1_SQRT_2, 0.98078525, 0.9238795, 0.8314696, 0.70710677, 0.5555702, 0.38268343, 0.19509023], 
    [FRAC_1_SQRT_2, 0.8314696, 0.38268343, -0.19509032, -0.70710677, -0.9807853, -0.9238795, -0.55557],
    [FRAC_1_SQRT_2, 0.5555702, -0.38268352, -0.9807853, -0.70710665, 0.19509041, 0.92387956, 0.83146936],
    [FRAC_1_SQRT_2, 0.19509023, -0.9238796, -0.55557, 0.707107, 0.83146936, -0.3826839, -0.9807852],
    [FRAC_1_SQRT_2, -0.19509032, -0.9238795, 0.5555704, 0.70710677, -0.8314698, -0.38268298, 0.9807854],
    [FRAC_1_SQRT_2, -0.55557036, -0.38268313, 0.98078525, -0.70710725, -0.19509022, 0.9238793, -0.8314696],
    [FRAC_1_SQRT_2, -0.83146966, 0.3826836, 0.19509007, -0.70710653, 0.9807853, -0.92387974, 0.55557114],
    [FRAC_1_SQRT_2, -0.9807853, 0.92387956, -0.8314698, 0.7071068, -0.55557084, 0.3826839, -0.19509155]
];
