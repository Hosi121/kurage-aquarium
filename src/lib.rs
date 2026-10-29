pub mod cli;
pub mod export;
pub mod model;
mod pipeline;
pub mod render;
pub mod scene;

pub type Vec3 = [f64; 3];
pub const BACKGROUND: [u8; 3] = [4, 9, 14];
pub const TAU: f64 = std::f64::consts::TAU;

pub fn mix(a: Vec3, b: Vec3, t: f64) -> Vec3 {
    std::array::from_fn(|i| a[i] * (1. - t) + b[i] * t)
}

pub fn scale(a: Vec3, t: f64) -> Vec3 {
    a.map(|v| v * t)
}

// Python/NumPy use a positive remainder and ties-to-even rounding.
pub fn modulo(x: f64, period: f64) -> f64 {
    x.rem_euclid(period)
}

pub fn byte(x: f64) -> u8 {
    x.round_ties_even().clamp(0., 255.) as u8
}
