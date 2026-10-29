//! Conservative bounds for the fixed anatomical model and its delayed path.
//! Reject only when an enclosing world AABB is outside a complete frustum
//! plane. Uncertain wrap/camera/numeric cases deliberately stay visible.
use crate::{
    TAU,
    scene::{Drifter, PERIOD},
};

// All aquarium samples (carry_swim=false) have y in [-1.1,3.5] and
// hypot(x,z) <= 1.8. Bell height <= .82*1.1+.11; filaments <=3.3+.035.
// Their |x| <= 1.035*1.023+.11+.19 <1.4 and |z| <1.0.
const HEAD: f64 = 1.1;
const TAIL: f64 = 3.5;
const RADIUS: f64 = 1.8;

pub(crate) fn offscreen(d: &Drifter, t: f64, width: usize, height: usize, max_swim: f64) -> bool {
    if !t.is_finite()
        || t.abs() > 1e6
        || width == 0
        || height == 0
        || !max_swim.is_finite()
        || d.size <= 0.
    {
        return false;
    }
    let bottom = 3.65 + 1.30 * d.size;
    let travel = bottom - (-4.20 - 5.20 * d.size);
    let stroke = 0.80 * d.size * max_swim / travel;
    // A sample follows a pose at t-delay, delay in [0,.95]. Interpolation
    // stays within the precomputed swimming table's extrema.
    let low = (t - 0.95) / PERIOD + d.entry - stroke - 1e-10;
    let high = t / PERIOD + d.entry + stroke + 1e-10;
    if low.floor() != high.floor() {
        return false;
    }
    let (lo, hi) = (low - low.floor(), high - low.floor());
    let middle = (lo + hi) / 2.;
    let half = (hi - lo) / 2.;
    let a = TAU * middle + d.curve;
    // sin/cos are 1-Lipschitz, so these cover the whole delayed interval.
    let cx = d.lane + d.lean * (middle - 0.5) + d.sway * a.sin();
    let rx = (d.lean.abs() + TAU * d.sway.abs()) * half;
    let cz = d.depth + 0.24 * (a + 0.9).sin();
    let rz = TAU * 0.24 * half;
    let tx = d.lean.abs() + TAU * d.sway.abs();
    let tz = TAU * 0.24;
    let tilt = (tx * tx + tz * tz).sqrt() / (travel * travel + tx * tx + tz * tz).sqrt();
    let tail_x = tx / (travel * travel + tx * tx).sqrt();
    let tail_z = tz / (travel * travel + tz * tz).sqrt();
    // The other two body axes span the plane perpendicular to the tail.
    let dx = d.size * (RADIUS + TAIL * tail_x);
    let dz = d.size * (RADIUS + TAIL * tail_z);
    let padding = 1e-7;
    let xmin = cx - rx - dx - padding;
    let xmax = cx + rx + dx + padding;
    let ymin = bottom - travel * hi - d.size * (HEAD + RADIUS * tilt) - padding;
    let ymax = bottom - travel * lo + d.size * (TAIL + RADIUS * tilt) + padding;
    let zmin = cz - rz - dz - padding;
    let zmax = cz + rz + dz + padding;
    if zmax >= 8.99 {
        return false;
    }
    let scale = 9. * width as f64 / 10.;
    let left = (width as f64 / 2. + 1.) / scale;
    let right = (width as f64 / 2.) / scale;
    let top = (height as f64 / 2. + 1.) / scale;
    let bottom = (height as f64 / 2.) / scale;
    // Raster splats can affect the viewport from [-1,width) x [-1,height).
    let distance = 9. - zmin;
    xmax + left * distance < 0.
        || xmin - right * distance >= 0.
        || ymax + top * distance < 0.
        || ymin - bottom * distance >= 0.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{DRIFTERS, Scene};

    #[test]
    fn anatomy_stays_inside_documented_enclosure() {
        let scene = Scene::aquarium();
        for (index, d) in DRIFTERS.iter().enumerate() {
            for step in 0..33 {
                let t = step as f64 * PERIOD / 32.;
                for point in scene.models[index].points(t * d.speed + d.pulse, false) {
                    assert!(point.xyz[1] >= -HEAD && point.xyz[1] <= TAIL);
                    assert!(point.xyz[0].hypot(point.xyz[2]) <= RADIUS);
                }
            }
        }
    }

    #[test]
    fn rejection_never_removes_samples_inside_splat_halo() {
        let scene = Scene::aquarium();
        let mut rejected = 0;
        for (index, d) in DRIFTERS.iter().enumerate() {
            for step in -2..=66 {
                let t = step as f64 * PERIOD / 64.;
                let heights = [64, 180, 360];
                let skips = heights.map(|h| {
                    offscreen(
                        d,
                        t,
                        320,
                        h,
                        scene.models[index].swimming.max_displacement(),
                    )
                });
                if !skips.iter().any(|&skip| skip) {
                    continue;
                }
                let points = scene.projected(index, t, 320, 180);
                for (height, skip) in heights.into_iter().zip(skips) {
                    if !skip {
                        continue;
                    }
                    rejected += 1;
                    assert!(
                        !points.iter().any(|p| {
                            let y = p.y + (height as f64 - 180.) / 2.;
                            p.x >= -1. && p.x < 320. && y >= -1. && y < height as f64
                        }),
                        "swimmer {index} time {t} height {height}"
                    );
                }
            }
        }
        assert!(
            rejected > 30,
            "bound never skipped enough samples to be useful"
        );
    }

    #[test]
    fn uncertain_inputs_do_not_skip_geometry() {
        for t in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e20] {
            assert!(!offscreen(&DRIFTERS[0], t, 320, 180, 0.135));
        }
    }
}
