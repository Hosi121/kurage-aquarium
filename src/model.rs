use crate::{TAU, Vec3, mix, modulo, scale};

pub const STROKE: f64 = 4.8;
const CYAN: Vec3 = [36. / 255., 185. / 255., 205. / 255.];
const ROSE: Vec3 = [194. / 255., 132. / 255., 166. / 255.];
const GOLD: Vec3 = [226. / 255., 174. / 255., 113. / 255.];
const VIOLET: Vec3 = [124. / 255., 113. / 255., 184. / 255.];

pub fn contraction(t: f64) -> f64 {
    let p = modulo(t, STROKE) / STROKE;
    let u = if p < 0.28 {
        p / 0.28
    } else {
        (p - 0.28) / 0.72
    };
    let smooth = u * u * u * (10. + u * (-15. + 6. * u));
    if p < 0.28 { smooth } else { 1. - smooth }
}

#[derive(Clone)]
pub struct Swimming {
    positions: Vec<f64>,
    speeds: Vec<f64>,
    max_position: f64,
}

impl Default for Swimming {
    fn default() -> Self {
        let count = 1024;
        let dt = STROKE / count as f64;
        let q: Vec<_> = (0..count).map(|i| contraction(i as f64 * dt)).collect();
        let squeeze: Vec<_> = (0..count)
            .map(|i| ((q[(i + 1) % count] - q[(i + count - 1) % count]) / (2. * dt)).max(0.))
            .collect();
        let mean = squeeze.iter().sum::<f64>() / count as f64;
        let force: Vec<_> = squeeze.iter().map(|s| 0.78 * (mean - s)).collect();
        let mut speeds = vec![0.; count];
        let mut velocity = 0.;
        for cycle in 0..7 {
            for i in 0..count {
                velocity += dt * (force[i] - 1.9 * velocity);
                if cycle == 6 {
                    speeds[i] = velocity;
                }
            }
        }
        let mean = speeds.iter().sum::<f64>() / count as f64;
        speeds.iter_mut().for_each(|v| *v -= mean);
        let mut positions = Vec::with_capacity(count + 1);
        positions.push(0.);
        let mut sum = 0.;
        for speed in &speeds {
            sum += speed * dt;
            positions.push(sum);
        }
        let low = positions.iter().copied().fold(f64::INFINITY, f64::min);
        let high = positions.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let gain = 0.27 / (high - low);
        positions
            .iter_mut()
            .for_each(|v| *v = (*v - (high + low) / 2.) * gain);
        speeds.iter_mut().for_each(|v| *v *= gain);
        speeds.push(speeds[0]);
        let max_position = positions.iter().map(|p| p.abs()).fold(0., f64::max) + 1e-12;
        Self {
            positions,
            speeds,
            max_position,
        }
    }
}

impl Swimming {
    pub fn max_displacement(&self) -> f64 {
        self.max_position
    }
    fn interpolate(&self, data: &[f64], t: f64) -> f64 {
        let time = modulo(t, STROKE);
        let dt = STROKE / 1024.;
        let i = ((time / dt) as usize).min(1023);
        // Match np.interp's slope-first operation order.
        data[i] + ((data[i + 1] - data[i]) / dt) * (time - i as f64 * dt)
    }
    pub fn position(&self, t: f64) -> f64 {
        self.interpolate(&self.positions, t)
    }
    pub fn speed(&self, t: f64) -> f64 {
        self.interpolate(&self.speeds, t)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub xyz: Vec3,
    pub light: Vec3,
}
struct Cap {
    xyz: Vec3,
    color: Vec3,
    weight: f64,
    profile: f64,
    rim: f64,
    edge: f64,
    scallop: f64,
}
struct Rim {
    cos: f64,
    sin: f64,
    shape: f64,
    y: f64,
}
struct Core {
    x: f64,
    z: f64,
    wave: f64,
    layer: f64,
    light: Vec3,
}
struct Arm {
    s: f64,
    u: f64,
    delay: f64,
    color: Vec3,
    fade: f64,
}
struct Thread {
    a: f64,
    s: f64,
    cos: f64,
    sin: f64,
    length: f64,
    delay: f64,
    rim: f64,
    scallop: f64,
    light: Vec3,
}

pub struct Jellyfish {
    pub swimming: Swimming,
    pub loop_period: Option<f64>,
    cap: Vec<Cap>,
    rim: Vec<Rim>,
    core: Vec<Core>,
    arms: Vec<Arm>,
    threads: Vec<Thread>,
    rim_energy: f64,
    arm_columns: usize,
    thread_steps: usize,
}

fn lin(i: usize, n: usize, a: f64, b: f64) -> f64 {
    if i + 1 == n {
        b
    } else {
        a + i as f64 * ((b - a) / (n - 1) as f64)
    }
}

impl Jellyfish {
    pub fn new(loop_period: Option<f64>, detail: usize) -> Self {
        assert!(detail > 0);
        let mut cap = Vec::new();
        for i in 0..(36 * detail + 1) {
            let u = lin(i, 36 * detail + 1, 0.04, std::f64::consts::FRAC_PI_2);
            for j in 0..(128 * detail) {
                let a = j as f64 * (TAU / (128 * detail) as f64);
                let radius = u.sin();
                let edge = radius.powi(12);
                let fresnel = (1. - (a.sin() * u.sin()).abs()).powi(2);
                let ribs = (a * 20. + u * 1.5).cos().max(0.).powi(18);
                cap.push(Cap {
                    xyz: [radius * a.cos(), -0.82 * u.cos(), 0.76 * radius * a.sin()],
                    color: scale(CYAN, 0.80 + 0.20 * edge),
                    weight: (0.032 + 0.095 * fresnel + 0.10 * ribs + 0.065 * edge)
                        * radius.max(0.12)
                        * ((37 * 128) as f64 / ((36 * detail + 1) * 128 * detail) as f64),
                    profile: 0.055 + 0.115 * radius.powi(4),
                    rim: 0.035 * (20. * a).cos(),
                    edge,
                    scallop: 1. + 0.023 * (20. * a).cos() * edge,
                });
            }
        }
        let rim = (0..560 * detail)
            .map(|i| {
                let a = i as f64 * (TAU / (560 * detail) as f64);
                Rim {
                    cos: a.cos(),
                    sin: a.sin(),
                    shape: 1. + 0.023 * (20. * a).cos(),
                    y: 0.035 * (20. * a).cos(),
                }
            })
            .collect();
        let mut core = Vec::new();
        for i in 0..130 * detail {
            let a = i as f64 * (TAU / (130 * detail) as f64);
            for j in 0..6 * detail + 1 {
                let layer = lin(j, 6 * detail + 1, -1., 1.);
                let r = 0.20 + 0.090 * (4. * a).cos() + 0.073 * layer;
                let blush = 0.22 * (0.5 + 0.5 * a.sin());
                let energy = 0.115
                    * (1. - 0.35 * layer * layer)
                    * ((130 * 7) as f64 / (130 * detail * (6 * detail + 1)) as f64);
                core.push(Core {
                    x: r * a.cos(),
                    z: 0.65 * r * a.sin(),
                    wave: 0.04 * (2. * a).sin(),
                    layer,
                    light: scale(mix(GOLD, ROSE, blush), energy),
                });
            }
        }
        let mut arms = Vec::new();
        for i in 0..169 * detail + 1 {
            let s = lin(i, 169 * detail + 1, 0., 1.);
            for j in 0..6 * detail + 1 {
                let u = lin(j, 6 * detail + 1, -1., 1.);
                let warmth = 0.35 * (-s * 4.5).exp();
                arms.push(Arm {
                    s,
                    u,
                    delay: 0.24 * s + 1.55 * s * s,
                    color: mix(mix(ROSE, VIOLET, s), GOLD, warmth),
                    fade: (1. - 0.55 * s)
                        * (0.65 + 0.35 * u.abs())
                        * ((170 * 7) as f64 / ((169 * detail + 1) * (6 * detail + 1)) as f64),
                });
            }
        }
        let mut threads = Vec::new();
        for i in 0..26 {
            let a = i as f64 * (TAU / 26.);
            for j in 0..114 * detail + 1 {
                let s = lin(j, 114 * detail + 1, 0., 1.);
                let shade = 0.5 + 0.5 * (a + 0.7).sin();
                let color = scale(mix(CYAN, VIOLET, s * 0.65), 0.58 + 0.42 * shade);
                let energy = (0.075 + 0.035 * (12. * s + a).sin().powi(2))
                    * (1. - 0.72 * s)
                    * (115. / (114 * detail + 1) as f64);
                threads.push(Thread {
                    a,
                    s,
                    cos: a.cos(),
                    sin: a.sin(),
                    length: 2.45 + 0.85 * (0.5 + 0.5 * (a * 2.3 + 1.).sin()),
                    delay: 0.35 * s + 2.1 * s * s,
                    rim: 0.035 * (20. * a).cos(),
                    scallop: 1. + 0.023 * (20. * a).cos() * (1. - s),
                    light: scale(color, energy),
                });
            }
        }
        Self {
            swimming: Swimming::default(),
            loop_period,
            cap,
            rim,
            core,
            arms,
            threads,
            rim_energy: 0.17 / detail as f64,
            arm_columns: 6 * detail + 1,
            thread_steps: 114 * detail + 1,
        }
    }

    pub fn angle(&self, t: f64, mut speed: f64) -> f64 {
        let mut t = t;
        if let Some(period) = self.loop_period {
            speed = (speed * period / TAU).round_ties_even().max(1.) * TAU / period;
            t = modulo(t, period);
        }
        t * speed
    }

    pub fn points(&self, t: f64, carry: bool) -> Vec<Point> {
        let mut result = Vec::new();
        self.points_into(t, carry, &mut result, &mut Vec::new());
        result
    }

    pub fn points_into(
        &self,
        t: f64,
        carry: bool,
        result: &mut Vec<Point>,
        pulse_cache: &mut Vec<(f64, f64)>,
    ) {
        result.clear();
        result.reserve(
            self.cap.len()
                + self.rim.len()
                + self.core.len()
                + 4 * self.arms.len()
                + self.threads.len(),
        );
        let squeeze = contraction(t);
        let height = 0.93 + 0.17 * squeeze;
        let rim_scale = 1.035 - 0.17 * squeeze;
        let lift = -0.075 * squeeze;
        let body = if carry { self.swimming.position(t) } else { 0. };
        for p in &self.cap {
            let profile = 1.035 - squeeze * p.profile;
            result.push(Point {
                xyz: [
                    p.xyz[0] * profile * p.scallop,
                    p.xyz[1] * height + (p.rim + lift) * p.edge,
                    p.xyz[2] * profile * p.scallop,
                ],
                light: scale(p.color, p.weight),
            });
        }
        for p in &self.rim {
            let r = rim_scale * p.shape;
            result.push(Point {
                xyz: [r * p.cos, p.y + lift, 0.76 * r * p.sin],
                light: scale(CYAN, self.rim_energy),
            });
        }
        for p in &self.core {
            result.push(Point {
                xyz: [p.x, -0.26 * height + 0.045 * p.layer + p.wave, p.z],
                light: p.light,
            });
        }
        let angle62 = self.angle(t, 0.62);
        let angle48 = self.angle(t, 0.48);
        let angle15 = self.angle(t, 0.15);
        let angle27 = self.angle(t, 0.27).sin();
        let angle_depth = self.angle(t, 0.62 * 0.79);
        for arm in 0..4 {
            let a = arm as f64 * std::f64::consts::FRAC_PI_2 + 0.35;
            for row in self.arms.chunks_exact(self.arm_columns) {
                let p = &row[0];
                let s = p.s;
                let carried = if carry {
                    self.swimming.position(t - p.delay) - body
                } else {
                    0.
                };
                let pulse = contraction(t - p.delay);
                let wave = 8.5 * s - angle62 + a;
                let fold = 42. * s - angle48 + a + 0.45 * pulse;
                let twist = 8. * s + a + angle15;
                let ribbon = (0.17 + 0.048 * fold.sin()) * (1. - 0.40 * s);
                let lag = 0.065 * (self.angle(t - p.delay, 0.27).sin() - angle27);
                let axis_x = 0.13 * a.cos() * (1. + s) + 0.14 * wave.sin() * s + 0.12 * s * s + lag;
                let depth = (8.5 * s + a) * 0.79 - angle_depth;
                let axis_z = 0.12 * a.sin() * (1. + s) + 0.11 * depth.cos() * s;
                let twist_cos = twist.cos();
                let twist_sin = twist.sin();
                for p in row {
                    let u = p.u;
                    let y = 0.04 - 0.04 * pulse
                        + s * (2.65 + arm as f64 * 0.14)
                        + 0.025 * (fold + u * 2.2).sin()
                        + carried * s;
                    let energy =
                        (0.040 + 0.080 * (0.5 + 0.5 * (fold + u * 3.).cos()).powi(5)) * p.fade;
                    result.push(Point {
                        xyz: [
                            axis_x + u * ribbon * twist_cos,
                            y,
                            axis_z + u * ribbon * twist_sin,
                        ],
                        light: scale(p.color, energy),
                    });
                }
            }
        }
        let angle40 = self.angle(t, 0.40);
        let angle_depth = self.angle(t, 0.40 * 0.8);
        pulse_cache.clear();
        pulse_cache.extend(self.threads[..self.thread_steps].iter().map(|p| {
            let carried = if carry {
                self.swimming.position(t - p.delay) - body
            } else {
                0.
            };
            (contraction(t - p.delay), carried)
        }));
        for (i, p) in self.threads.iter().enumerate() {
            let (s, a) = (p.s, p.a);
            let (pulse, carried) = pulse_cache[i % self.thread_steps];
            let radius = (1.035 - 0.17 * pulse) * (1. - 0.20 * s) * p.scallop;
            let wave = 7. * s + a * 2.5 - angle40;
            let depth = (7. * s + a * 2.5) * 0.8 - angle_depth;
            result.push(Point {
                xyz: [
                    p.cos * radius + 0.11 * wave.sin() * s + 0.19 * s * s,
                    s * p.length + p.rim - 0.075 * pulse + carried * s,
                    0.76 * p.sin * radius + 0.10 * depth.cos() * s,
                ],
                light: p.light,
            });
        }
    }
}
