//! Cross-machine transcendentals for the simulation.
//!
//! Platform `sin` / `cos` / `atan2` come from glibc or the Windows CRT and do
//! not match. These wrappers call the pure-Rust `libm` port. `sqrt`, `pow`,
//! and `exp` go through the same crate so a later edit cannot pick up a
//! hardware intrinsic by accident. The build script rejects `target-cpu=native`
//! and the `fma` target feature; rustc does not contract `a * b + c` into an
//! FMA unless code calls `mul_add`.

pub fn sin(value: f32) -> f32 {
    libm::sinf(value)
}

pub fn cos(value: f32) -> f32 {
    libm::cosf(value)
}

pub fn atan2(y: f32, x: f32) -> f32 {
    libm::atan2f(y, x)
}

pub fn sqrt(value: f32) -> f32 {
    libm::sqrtf(value)
}

pub fn powf(base: f32, exponent: f32) -> f32 {
    libm::powf(base, exponent)
}

pub fn exp(value: f32) -> f32 {
    libm::expf(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrappers_match_libm_bits() {
        let samples = [-1.25_f32, 0.0, 0.35, 1.0, 2.5];
        for value in samples {
            assert_eq!(sin(value).to_bits(), libm::sinf(value).to_bits());
            assert_eq!(cos(value).to_bits(), libm::cosf(value).to_bits());
            assert_eq!(
                sqrt(value.abs()).to_bits(),
                libm::sqrtf(value.abs()).to_bits()
            );
            assert_eq!(exp(value).to_bits(), libm::expf(value).to_bits());
            assert_eq!(
                powf(value.abs() + 0.25, 1.5).to_bits(),
                libm::powf(value.abs() + 0.25, 1.5).to_bits()
            );
            assert_eq!(
                atan2(value, 0.5).to_bits(),
                libm::atan2f(value, 0.5).to_bits()
            );
        }
    }
}
