use crate::{
    TAU, Vec3,
    model::{Jellyfish, Point},
    modulo,
};

pub const PERIOD: f64 = 38.4;
#[derive(Clone, Copy)]
pub struct Drifter {
    pub lane: f64,
    pub depth: f64,
    pub size: f64,
    pub speed: f64,
    pub pulse: f64,
    pub entry: f64,
    pub sway: f64,
    pub curve: f64,
    pub lean: f64,
    pub spin: f64,
    pub tint: Vec3,
    pub detail: usize,
}
pub const DRIFTERS: [Drifter; 9] = [
    Drifter {
        lane: -2.75,
        depth: 0.65,
        size: 0.85,
        speed: 1.,
        pulse: 0.35,
        entry: 0.22,
        sway: 0.95,
        curve: 0.10,
        lean: -2.20,
        spin: 0.10,
        tint: [1., 1., 1.],
        detail: 1,
    },
    Drifter {
        lane: 2.65,
        depth: 0.25,
        size: 0.68,
        speed: 7. / 8.,
        pulse: 2.45,
        entry: 0.345,
        sway: 0.90,
        curve: 2.20,
        lean: 2.20,
        spin: 1.15,
        tint: [0.96, 0.98, 1.08],
        detail: 1,
    },
    Drifter {
        lane: -0.35,
        depth: -0.85,
        size: 0.48,
        speed: 9. / 8.,
        pulse: 1.15,
        entry: 0.470,
        sway: 0.53,
        curve: 4.10,
        lean: -0.70,
        spin: -0.85,
        tint: [1.09, 0.94, 1.10],
        detail: 1,
    },
    Drifter {
        lane: 1.15,
        depth: -1.45,
        size: 0.35,
        speed: 3. / 4.,
        pulse: 3.80,
        entry: 0.595,
        sway: 0.45,
        curve: 1.20,
        lean: 0.80,
        spin: 2.00,
        tint: [0.92, 1.05, 1.08],
        detail: 1,
    },
    Drifter {
        lane: -3.60,
        depth: -0.65,
        size: 0.43,
        speed: 5. / 4.,
        pulse: 2.80,
        entry: 0.720,
        sway: 0.53,
        curve: 5.20,
        lean: -0.90,
        spin: -1.40,
        tint: [1.10, 0.93, 1.12],
        detail: 1,
    },
    Drifter {
        lane: 0.75,
        depth: 0.15,
        size: 0.54,
        speed: 1.,
        pulse: 4.10,
        entry: 0.845,
        sway: 0.88,
        curve: 3.30,
        lean: 1.40,
        spin: 0.65,
        tint: [0.98, 1.02, 1.05],
        detail: 1,
    },
    Drifter {
        lane: -1.45,
        depth: -1.55,
        size: 0.32,
        speed: 7. / 8.,
        pulse: 1.80,
        entry: 0.970,
        sway: 0.40,
        curve: 2.80,
        lean: 0.30,
        spin: -1.80,
        tint: [0.94, 1.04, 1.08],
        detail: 1,
    },
    Drifter {
        lane: 3.55,
        depth: -1.05,
        size: 0.39,
        speed: 9. / 8.,
        pulse: 0.75,
        entry: 0.095,
        sway: 0.47,
        curve: 0.80,
        lean: -0.65,
        spin: 1.65,
        tint: [1.05, 0.96, 1.10],
        detail: 1,
    },
    Drifter {
        lane: -0.45,
        depth: 0.95,
        size: 3.20,
        speed: 1. / 2.,
        pulse: 0.65,
        entry: 0.255,
        sway: 0.55,
        curve: 0.40,
        lean: -1.80,
        spin: 0.35,
        tint: [0.98, 1.02, 1.04],
        detail: 2,
    },
];

#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub center: Vec3,
    pub basis: [Vec3; 3],
    pub velocity: Vec3,
}

pub struct Scene {
    pub models: Vec<Jellyfish>,
    pub aquarium: bool,
}
#[derive(Clone, Copy)]
pub struct Projected {
    pub x: f64,
    pub y: f64,
    pub light: Vec3,
}

/// One rendering worker owns and reuses these buffers across frames.
#[derive(Default)]
pub struct LightWorkspace {
    points: Vec<Point>,
    projected: Vec<Projected>,
    pass: Vec<Vec3>,
    pulse: Vec<(f64, f64)>,
}

impl Scene {
    pub fn aquarium() -> Self {
        Self {
            models: DRIFTERS
                .iter()
                .map(|d| Jellyfish::new(Some(PERIOD * d.speed), d.detail))
                .collect(),
            aquarium: true,
        }
    }
    pub fn single(period: Option<f64>) -> Self {
        Self {
            models: vec![Jellyfish::new(period, 1)],
            aquarium: false,
        }
    }
    pub fn kinematics(&self, index: usize, t: f64) -> Pose {
        let d = DRIFTERS[index];
        let model = &self.models[index];
        let local = t * d.speed + d.pulse;
        let bottom = 3.65 + 1.30 * d.size;
        let top = -4.20 - 5.20 * d.size;
        let travel = bottom - top;
        let raw = t / PERIOD + d.entry - 0.80 * d.size * model.swimming.position(local) / travel;
        let u = modulo(raw, 1.);
        let du = 1. / PERIOD - 0.80 * d.size * d.speed * model.swimming.speed(local) / travel;
        let a = TAU * u + d.curve;
        let center = [
            d.lane + d.lean * (u - 0.5) + d.sway * a.sin(),
            bottom - travel * u,
            d.depth + 0.24 * (a + 0.9).sin(),
        ];
        let tx = d.lean + TAU * d.sway * a.cos();
        let tz = TAU * 0.24 * (a + 0.9).cos();
        let length = (tx * tx + travel * travel + tz * tz).sqrt();
        let (tail_x, tail_y, tail_z) = (-tx / length, travel / length, -tz / length);
        let right_length = (tail_y * tail_y + tail_x * tail_x).sqrt();
        let (right_x, right_y) = (tail_y / right_length, -tail_x / right_length);
        let (normal_x, normal_y, normal_z) = (
            right_y * tail_z,
            -right_x * tail_z,
            right_x * tail_y - right_y * tail_x,
        );
        let spin = d.spin + 0.12 * (TAU * t / PERIOD + d.curve).sin();
        let (s, c) = spin.sin_cos();
        Pose {
            center,
            basis: [
                [
                    right_x * c + normal_x * s,
                    tail_x,
                    normal_x * c - right_x * s,
                ],
                [
                    right_y * c + normal_y * s,
                    tail_y,
                    normal_y * c - right_y * s,
                ],
                [normal_z * s, tail_z, normal_z * c],
            ],
            velocity: [tx * du, -travel * du, tz * du],
        }
    }

    pub fn projected(&self, index: usize, t: f64, width: usize, height: usize) -> Vec<Projected> {
        let mut workspace = LightWorkspace::default();
        self.projected_into(index, t, width, height, &mut workspace);
        workspace.projected
    }

    fn projected_into(
        &self,
        index: usize,
        t: f64,
        width: usize,
        height: usize,
        workspace: &mut LightWorkspace,
    ) {
        let d = DRIFTERS[index];
        self.models[index].points_into(
            t * d.speed + d.pulse,
            false,
            &mut workspace.points,
            &mut workspace.pulse,
        );
        let current = self.kinematics(index, t);
        let unit = width as f64 / 10.;
        let density = (unit * d.size / 11.5).powf(1.6) * (8. / 2_f64.powf(1.6));
        let fog = 0.61 + 0.35 * ((current.center[2] + 1.8) / 2.8).clamp(0., 1.);
        workspace.projected.extend(workspace.points.iter().map(|p| {
            let delay = 0.95 * (p.xyz[1] / 3.2).clamp(0., 1.).powf(1.4);
            let pose = if delay == 0. {
                current
            } else {
                self.kinematics(index, t - delay)
            };
            let world: Vec3 = std::array::from_fn(|j| {
                pose.center[j]
                    + d.size
                        * ((pose.basis[j][0] * p.xyz[0] + pose.basis[j][1] * p.xyz[1])
                            + pose.basis[j][2] * p.xyz[2])
            });
            let perspective = 9. / (9. - world[2]);
            let highlight = 0.88 + 0.12 * (world[2] - current.center[2] + 0.5).clamp(0., 1.);
            Projected {
                x: width as f64 / 2. + world[0] * perspective * unit,
                y: height as f64 / 2. + world[1] * perspective * unit,
                light: std::array::from_fn(|j| {
                    (p.light[j] * (density * fog)) * d.tint[j] * highlight
                }),
            }
        }));
    }

    pub fn light_frame(&self, columns: usize, rows: usize, t: f64, ascii: bool) -> Vec<Vec3> {
        let mut pixels = Vec::new();
        self.light_frame_into(
            columns,
            rows,
            t,
            ascii,
            &mut LightWorkspace::default(),
            &mut pixels,
        );
        pixels
    }

    pub fn light_frame_into(
        &self,
        columns: usize,
        rows: usize,
        t: f64,
        ascii: bool,
        workspace: &mut LightWorkspace,
        pixels: &mut Vec<Vec3>,
    ) {
        let (columns, rows) = (columns.max(1), rows.max(1));
        workspace.projected.clear();
        if self.aquarium {
            let (width, height) = (columns * 2, rows * 4);
            workspace.projected.reserve(160_000);
            for (i, model) in self.models.iter().enumerate() {
                if crate::visibility::offscreen(
                    &DRIFTERS[i],
                    t,
                    width,
                    height,
                    model.swimming.max_displacement(),
                ) {
                    continue;
                }
                self.projected_into(i, t, width, height, workspace);
            }
            rasterize_into(
                &mut workspace.projected,
                width,
                height,
                pixels,
                &mut workspace.pass,
            );
        } else {
            let model = &self.models[0];
            let fine = !ascii;
            let (width, height) = if fine {
                (columns * 2, rows * 4)
            } else {
                (columns, rows)
            };
            let compact = rows < 14 && columns >= 30;
            let aspect = if fine { 0.96 } else { 0.48 };
            let gain = if fine { 8. / 2_f64.powf(1.6) } else { 1. };
            let yaw = 0.20 * model.angle(t, 0.17).sin();
            let (sy, cy) = yaw.sin_cos();
            let tilt = (if compact { 1.24 } else { 0.26 }) + 0.045 * model.angle(t, 0.21).sin();
            let (st, ct) = tilt.sin_cos();
            let (world_width, world_height, center_x, center_y) = if compact {
                (4.7, 2.7, 1., 0.05)
            } else {
                (3.1, 4.6, 0.27, 1.16)
            };
            let scale =
                0.94 * (width as f64 / world_width).min(height as f64 / (world_height * aspect));
            let drift = model.angle(t, 0.27).sin() * scale * 0.045;
            let energy = (scale / 11.5).powf(1.6) * gain;
            let swim = model.swimming.position(t);
            model.points_into(t, true, &mut workspace.points, &mut workspace.pulse);
            workspace.projected.extend(workspace.points.iter().map(|p| {
                let x = p.xyz[0] * cy + p.xyz[2] * sy;
                let z = p.xyz[2] * cy - p.xyz[0] * sy;
                let y = p.xyz[1] + swim;
                let (x, y) = (x * ct + y * st, y * ct - x * st);
                let y = y * 0.94 + z * 0.34;
                let perspective = 5. / (5. - z);
                let highlight = 0.86 + 0.14 * (z + 0.5).clamp(0., 1.);
                Projected {
                    x: (x * perspective - center_x) * scale + width as f64 * 0.5 + drift,
                    y: (y * perspective - center_y) * scale * aspect + height as f64 * 0.5,
                    light: p.light.map(|v| (v * energy) * highlight),
                }
            }));
            rasterize_into(
                &mut workspace.projected,
                width,
                height,
                pixels,
                &mut workspace.pass,
            );
        }
    }
}

pub fn rasterize(mut points: Vec<Projected>, width: usize, height: usize) -> Vec<Vec3> {
    let mut pixels = Vec::new();
    rasterize_into(&mut points, width, height, &mut pixels, &mut Vec::new());
    pixels
}

fn rasterize_into(
    points: &mut Vec<Projected>,
    width: usize,
    height: usize,
    pixels: &mut Vec<Vec3>,
    pass: &mut Vec<Vec3>,
) {
    points.retain(|p| p.x >= -1. && p.x < width as f64 && p.y >= -1. && p.y < height as f64);
    pixels.resize(width * height, [0.; 3]);
    pixels.fill([0.; 3]);
    pass.resize(width * height, [0.; 3]);
    // Separate coverage passes preserve the source's accumulation order.
    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        pass.fill([0.; 3]);
        for p in points.iter() {
            let (ix, iy) = (p.x.floor() as isize, p.y.floor() as isize);
            let (x, y) = (ix + dx, iy + dy);
            if x < 0 || y < 0 || x >= width as isize || y >= height as isize {
                continue;
            }
            let (fx, fy) = (p.x - ix as f64, p.y - iy as f64);
            let weight =
                (if dx == 0 { 1. - fx } else { fx }) * (if dy == 0 { 1. - fy } else { fy });
            let target = &mut pass[y as usize * width + x as usize];
            for (j, v) in target.iter_mut().enumerate() {
                *v += p.light[j] * weight;
            }
        }
        for (pixel, add) in pixels.iter_mut().zip(pass.iter()) {
            for j in 0..3 {
                pixel[j] += add[j];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_scratch_buffers_retain_storage_after_warmup() {
        let scene = Scene::aquarium();
        let mut workspace = LightWorkspace::default();
        let mut pixels = Vec::new();
        scene.light_frame_into(160, 45, 0., false, &mut workspace, &mut pixels);
        let storage = (
            workspace.points.as_ptr(),
            workspace.projected.as_ptr(),
            workspace.pass.as_ptr(),
            workspace.pulse.as_ptr(),
            pixels.as_ptr(),
        );
        for t in [1. / 30., 2. / 30., 12.8, 19.2, 38.4] {
            scene.light_frame_into(160, 45, t, false, &mut workspace, &mut pixels);
            assert_eq!(
                storage,
                (
                    workspace.points.as_ptr(),
                    workspace.projected.as_ptr(),
                    workspace.pass.as_ptr(),
                    workspace.pulse.as_ptr(),
                    pixels.as_ptr()
                )
            );
        }
    }
}
