use crate::{Vec3, byte};
use std::fmt::Write;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub glyphs: Vec<char>,
    pub colors: Vec<[u8; 3]>,
}
#[derive(Default)]
pub struct Shader {
    previous: Vec<Vec3>,
    dots: Vec<bool>,
    time: Option<f64>,
    shape: Option<(usize, usize, bool)>,
    brightness: Vec<f64>,
    strength: Vec<f64>,
    active: Vec<bool>,
}

fn threshold(x: usize, y: usize) -> f64 {
    // The source expands [[0,2],[3,1]] to an 8x8 Bayer coverage matrix.
    const BASE: [[usize; 2]; 2] = [[0, 2], [3, 1]];
    let value = 16 * BASE[y % 2][x % 2]
        + 4 * BASE[(y / 2) % 2][(x / 2) % 2]
        + BASE[(y / 4) % 2][(x / 4) % 2];
    0.025 + 0.67 * (value as f64 + 0.5) / 64.
}

impl Shader {
    pub fn shade(
        &mut self,
        mut pixels: Vec<Vec3>,
        width: usize,
        height: usize,
        t: f64,
        ascii: bool,
        temporal: bool,
    ) -> Frame {
        let mut result = Frame::default();
        self.shade_into(&mut pixels, width, height, t, ascii, temporal, &mut result);
        result
    }

    #[allow(clippy::too_many_arguments)]
    pub fn shade_into(
        &mut self,
        pixels: &mut [Vec3],
        width: usize,
        height: usize,
        t: f64,
        ascii: bool,
        temporal: bool,
        result: &mut Frame,
    ) {
        let mut history = false;
        if temporal {
            let dt = self.time.map_or(0., |old| t - old);
            if self.shape == Some((width, height, ascii))
                && self.previous.len() == pixels.len()
                && dt > 0.
                && dt < 0.25
            {
                let blend = -(-dt / 0.028).exp_m1();
                for (p, old) in pixels.iter_mut().zip(&mut self.previous) {
                    for j in 0..3 {
                        p[j] = old[j] + blend * (p[j] - old[j]);
                        old[j] = p[j];
                    }
                }
                history = self.dots.len() == pixels.len();
            } else {
                self.previous.resize(pixels.len(), [0.; 3]);
                self.previous.copy_from_slice(pixels);
            }
            self.time = Some(t);
            self.shape = Some((width, height, ascii));
        }
        let count = pixels.len();
        self.brightness.clear();
        self.strength.clear();
        self.brightness.reserve(count);
        self.strength.reserve(count);
        for p in pixels.iter() {
            let s = p[0].max(p[1]).max(p[2]);
            self.strength.push(s);
            self.brightness.push(1. - (-s * 0.27).exp());
        }
        let brightness = &self.brightness;
        let strength = &self.strength;
        result.width = width;
        result.height = height;
        result.glyphs.resize(width * height, ' ');
        result.colors.resize(width * height, [0; 3]);
        if ascii {
            let ramp: Vec<_> = " .,:-=+*#".chars().collect();
            for i in 0..count {
                result.glyphs[i] = ramp
                    [((brightness[i].powf(0.75) * ramp.len() as f64) as usize).min(ramp.len() - 1)];
                result.colors[i] = std::array::from_fn(|j| {
                    byte(
                        pixels[i][j] / strength[i].max(0.0001)
                            * (44. + 180. * brightness[i].powf(0.45)),
                    )
                });
            }
            return;
        }
        let dot_width = width * 2;
        self.active.resize(count, false);
        let dots = &mut self.active;
        for y in 0..height * 4 {
            for x in 0..dot_width {
                let i = y * dot_width + x;
                let deadband = if history {
                    if self.dots[i] { -0.012 } else { 0.012 }
                } else {
                    0.
                };
                dots[i] = brightness[i] > threshold(x, y) + deadband;
            }
        }
        const BITS: [[u16; 2]; 4] = [[1, 8], [2, 16], [4, 32], [64, 128]];
        for y in 0..height {
            for x in 0..width {
                let mut bits = 0;
                let mut weight_sum = 0.;
                let mut rgb = [0.; 3];
                for (dy, row) in BITS.iter().enumerate() {
                    for (dx, bit) in row.iter().enumerate() {
                        let i = (y * 4 + dy) * dot_width + x * 2 + dx;
                        if !dots[i] {
                            continue;
                        }
                        bits += bit;
                        let weight = brightness[i] + 0.03;
                        weight_sum += weight;
                        for j in 0..3 {
                            let color = byte(
                                pixels[i][j] / strength[i].max(0.0001)
                                    * (44. + 180. * brightness[i].powf(0.45)),
                            );
                            rgb[j] += color as f64 * weight;
                        }
                    }
                }
                let i = y * width + x;
                if bits != 0 {
                    result.glyphs[i] = char::from_u32(0x2800 + bits as u32).unwrap();
                    result.colors[i] = rgb.map(|v| byte(v / weight_sum.max(0.0001)));
                } else {
                    result.glyphs[i] = ' ';
                    result.colors[i] = [0; 3];
                }
            }
        }
        if temporal {
            std::mem::swap(&mut self.dots, &mut self.active);
        }
    }
}

pub fn encode(frame: &Frame, color: bool, previous: Option<&Frame>) -> String {
    let mut output = String::from("\x1b[H");
    for y in 0..frame.height {
        let range = y * frame.width..(y + 1) * frame.width;
        let (first, last) = if let Some(old) = previous {
            let positions: Vec<_> = range
                .clone()
                .filter(|&i| {
                    (frame.glyphs[i] != old.glyphs[i] || frame.colors[i] != old.colors[i])
                        && (frame.glyphs[i] != ' ' || old.glyphs[i] != ' ')
                })
                .collect();
            if positions.is_empty() {
                continue;
            }
            (
                positions[0] - range.start,
                positions[positions.len() - 1] - range.start + 1,
            )
        } else {
            (0, frame.width)
        };
        write!(output, "\x1b[{};{}H", y + 1, first + 1).unwrap();
        let mut active = None;
        for x in first..last {
            let i = y * frame.width + x;
            if color && frame.glyphs[i] != ' ' {
                let rgb = frame.colors[i].map(|v| (v / 4) * 4);
                if active != Some(rgb) {
                    write!(output, "\x1b[38;2;{};{};{}m", rgb[0], rgb[1], rgb[2]).unwrap();
                    active = Some(rgb);
                }
            }
            output.push(frame.glyphs[i]);
        }
    }
    output
}
