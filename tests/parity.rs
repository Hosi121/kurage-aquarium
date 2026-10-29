use kurage_aquarium::{
    export::Painter,
    model::Jellyfish,
    render::{Shader, encode},
    scene::{DRIFTERS, PERIOD, Scene},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

fn oracle() -> &'static Value {
    static ORACLE: OnceLock<Value> = OnceLock::new();
    ORACLE
        .get_or_init(|| serde_json::from_str(include_str!("fixtures/python-v0.3.0.json")).unwrap())
}
fn array(v: &Value) -> &Vec<Value> {
    v.as_array().unwrap()
}
fn number(v: &Value) -> f64 {
    v.as_f64().unwrap()
}
fn usize_value(v: &Value) -> usize {
    v.as_u64().unwrap() as usize
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-12, "{a:.17} != {b:.17}");
}

#[test]
fn python_frame_and_ansi_parity() {
    assert!(!array(&oracle()["series"]).is_empty());
    for case in array(&oracle()["series"]) {
        let scene = match case["scene"].as_str().unwrap() {
            "aquarium" => Scene::aquarium(),
            "single" => Scene::single(None),
            _ => Scene::single(Some(19.2)),
        };
        let (w, h) = (usize_value(&case["columns"]), usize_value(&case["rows"]));
        let ascii = case["ascii"].as_bool().unwrap();
        let mut shader = Shader::default();
        let mut previous = None;
        for expected in array(&case["frames"]) {
            let t = number(&expected["time"]);
            let actual = shader.shade(scene.light_frame(w, h, t, ascii), w, h, t, ascii, true);
            let glyphs: String = actual.glyphs.iter().collect();
            assert_eq!(
                glyphs,
                expected["glyphs"].as_str().unwrap(),
                "scene={} size={w}x{h} ascii={ascii} time={t}",
                case["scene"]
            );
            for (i, color) in actual.colors.iter().enumerate() {
                let target: [u8; 3] =
                    std::array::from_fn(|j| expected["colors"][i][j].as_u64().unwrap() as u8);
                assert_eq!(
                    *color, target,
                    "scene={} size={w}x{h} ascii={ascii} time={t} cell={i}",
                    case["scene"]
                );
            }
            assert_eq!(
                encode(&actual, true, None),
                expected["ansi"].as_str().unwrap()
            );
            assert_eq!(
                encode(&actual, true, previous.as_ref()),
                expected["ansi_delta"].as_str().unwrap()
            );
            previous = Some(actual);
        }
    }
}

#[test]
fn python_points_and_pose_parity() {
    let scene = Scene::aquarium();
    for expected in array(&oracle()["poses"]) {
        let pose = scene.kinematics(usize_value(&expected["index"]), number(&expected["time"]));
        for j in 0..3 {
            near(pose.center[j], number(&expected["center"][j]));
            near(pose.velocity[j], number(&expected["velocity"][j]));
            for k in 0..3 {
                near(pose.basis[j][k], number(&expected["basis"][j][k]));
            }
        }
    }
    for expected in array(&oracle()["points"]) {
        let model = Jellyfish::new(Some(19.2), usize_value(&expected["detail"]));
        let points = model.points(number(&expected["time"]), false);
        assert_eq!(points.len(), usize_value(&expected["count"]));
        for (n, index) in array(&expected["indices"]).iter().enumerate() {
            let point = points[usize_value(index)];
            for j in 0..3 {
                near(point.xyz[j], number(&expected["xyz"][n][j]));
                near(point.light[j], number(&expected["light"][n][j]));
            }
        }
    }
}

#[test]
fn equal_area_resize_resets_temporal_history() {
    for case in array(&oracle()["resizes"]) {
        let scene = match case["scene"].as_str().unwrap() {
            "aquarium" => Scene::aquarium(),
            "single" => Scene::single(None),
            _ => Scene::single(Some(19.2)),
        };
        let ascii = case["ascii"].as_bool().unwrap();
        let mut shader = Shader::default();
        for expected in array(&case["frames"]) {
            let (w, h, t) = (
                usize_value(&expected["width"]),
                usize_value(&expected["height"]),
                number(&expected["time"]),
            );
            let frame = shader.shade(scene.light_frame(w, h, t, ascii), w, h, t, ascii, true);
            assert_eq!(
                frame.glyphs.iter().collect::<String>(),
                expected["glyphs"].as_str().unwrap()
            );
            for (i, color) in frame.colors.iter().enumerate() {
                let target: [u8; 3] =
                    std::array::from_fn(|j| expected["colors"][i][j].as_u64().unwrap() as u8);
                assert_eq!(*color, target, "{} {w}x{h} time={t}", case["scene"]);
            }
        }
    }
}

#[test]
fn test_cached_glyphs_match_text_drawing() {
    for single in [false, true] {
        let scene = if single {
            Scene::single(Some(19.2))
        } else {
            Scene::aquarium()
        };
        let (w, h) = if single { (120, 50) } else { (160, 45) };
        let painter = Painter::new(single, None).unwrap();
        let mut shader = Shader::default();
        for i in -18..0 {
            let t = i as f64 / 30.;
            shader.shade(scene.light_frame(w, h, t, false), w, h, t, false, true);
        }
        for expected in array(&oracle()["glyphs"])
            .iter()
            .filter(|v| v["single"].as_bool() == Some(single))
        {
            let t = number(&expected["time"]);
            let frame = shader.shade(scene.light_frame(w, h, t, false), w, h, t, false, true);
            assert_eq!(
                frame.glyphs.iter().collect::<String>(),
                expected["glyphs"].as_str().unwrap()
            );
            let actual = format!("{:x}", Sha256::digest(painter.draw(&frame)));
            assert_eq!(
                actual,
                expected["rgb_sha256"].as_str().unwrap(),
                "single={single} time={t}"
            );
        }
    }
}

#[test]
fn test_heading_tracks_forward_motion() {
    let scene = Scene::aquarium();
    for index in 0..DRIFTERS.len() {
        for i in 0..257 {
            let p = scene.kinematics(index, i as f64 * PERIOD / 256.);
            let speed = p.velocity.iter().map(|v| v * v).sum::<f64>().sqrt();
            let alignment = (0..3).map(|j| -p.basis[j][1] * p.velocity[j]).sum::<f64>() / speed;
            assert!(alignment > 0.999999);
            assert!(p.velocity[1] < 0.);
            for j in 0..3 {
                for k in 0..3 {
                    near(
                        (0..3).map(|a| p.basis[a][j] * p.basis[a][k]).sum(),
                        if j == k { 1. } else { 0. },
                    );
                }
            }
        }
    }
}

#[test]
fn custom_fonts_overlap_and_clipping_match_pillow() {
    use kurage_aquarium::render::Frame;
    let font =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/DejaVuSans.ttf");
    for case in array(&oracle()["painters"]) {
        let painter = Painter::with_font(
            usize_value(&case["cell_width"]),
            usize_value(&case["cell_height"]),
            usize_value(&case["size"]) as u32,
            case["offset"].as_i64().unwrap() as i32,
            Some(&font),
        )
        .unwrap();
        let frame = Frame {
            width: 32,
            height: 16,
            glyphs: case["glyphs"].as_str().unwrap().chars().collect(),
            colors: array(&case["colors"])
                .iter()
                .map(|v| std::array::from_fn(|j| v[j].as_u64().unwrap() as u8))
                .collect(),
        };
        let actual = format!("{:x}", Sha256::digest(painter.draw(&frame)));
        assert_eq!(
            actual,
            case["rgb_sha256"].as_str().unwrap(),
            "font size={} offset={}",
            case["size"],
            case["offset"]
        );
    }
}

#[test]
fn test_recycling_is_invisible() {
    let scene = Scene::aquarium();
    for (index, d) in DRIFTERS.iter().enumerate() {
        let travel = (3.65 + 1.30 * d.size) - (-4.20 - 5.20 * d.size);
        let mut low = 0.;
        let mut high = PERIOD;
        for _ in 0..40 {
            let mid = (low + high) / 2.;
            let progress = mid / PERIOD + d.entry
                - 0.8
                    * d.size
                    * scene.models[index]
                        .swimming
                        .position(mid * d.speed + d.pulse)
                    / travel;
            if progress < 1. {
                low = mid;
            } else {
                high = mid;
            }
        }
        let reset = (low + high) / 2.;
        for t in [reset - 1. / 30., reset, reset + 1. / 30.] {
            assert!(
                !scene
                    .projected(index, t, 320, 180)
                    .iter()
                    .any(|p| p.x >= 0. && p.x < 320. && p.y >= 0. && p.y < 180.)
            );
        }
    }
}

#[test]
fn test_loop_has_no_boundary_jump() {
    let scene = Scene::aquarium();
    let mut shader = Shader::default();
    let start = shader.shade(
        scene.light_frame(80, 23, 0., false),
        80,
        23,
        0.,
        false,
        false,
    );
    let end = shader.shade(
        scene.light_frame(80, 23, PERIOD, false),
        80,
        23,
        PERIOD,
        false,
        false,
    );
    assert_eq!(start, end);
}

#[test]
fn test_standalone_modes_and_small_viewports() {
    let scene = Scene::single(None);
    for ascii in [true, false] {
        for (w, h) in [(1, 1), (8, 5), (38, 30), (80, 8)] {
            let frame = Shader::default().shade(
                scene.light_frame(w, h, 1.6, ascii),
                w,
                h,
                1.6,
                ascii,
                false,
            );
            assert_eq!(frame.glyphs.len(), w * h);
            assert_eq!(frame.colors.len(), w * h);
            assert!(frame.glyphs.iter().all(|&c| if ascii {
                c.is_ascii()
            } else {
                c == ' ' || (0x2801..=0x28ff).contains(&(c as u32))
            }));
        }
    }
}

#[test]
fn test_closeup_detail_preserves_light_and_color() {
    let ordinary = Jellyfish::new(None, 1);
    let detailed = Jellyfish::new(None, 2);
    for t in [0., 1.6, 3.2] {
        let a = ordinary.points(t, true);
        let b = detailed.points(t, true);
        assert!(b.len() > 2 * a.len());
        for j in 0..3 {
            let ea = a.iter().map(|p| p.light[j]).sum::<f64>();
            let eb = b.iter().map(|p| p.light[j]).sum::<f64>();
            assert!((ea - eb).abs() / ea < 0.01);
        }
    }
}

#[test]
fn test_parallel_rendering_preserves_temporal_filter() {
    use rayon::prelude::*;
    let scene = Scene::aquarium();
    let times = [-1. / 30., 0., 1. / 30., 2. / 30., 38.4, 38.4 + 1. / 30.];
    let expected: Vec<_> = times
        .iter()
        .map(|&t| scene.light_frame(38, 22, t, false))
        .collect();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let actual: Vec<_> = pool.install(|| {
        times
            .par_iter()
            .map(|&t| scene.light_frame(38, 22, t, false))
            .collect()
    });
    assert_eq!(expected, actual);
    let mut a = Shader::default();
    let mut b = Shader::default();
    for ((&t, x), y) in times.iter().zip(expected).zip(actual) {
        assert_eq!(
            a.shade(x, 38, 22, t, false, true),
            b.shade(y, 38, 22, t, false, true)
        );
    }
}

#[test]
fn test_shared_memory_cleanup_on_close_and_error() {
    // Shared memory was removed. Retained frames own their buffers after the
    // worker pool is dropped, including after a task's early failure.
    let scene = Scene::single(Some(19.2));
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let retained = pool.install(|| scene.light_frame(8, 5, 0., false));
    let reference = retained.clone();
    let failed: Result<(), &str> = pool.install(|| Err("fixture failure"));
    assert!(failed.is_err());
    drop(pool);
    assert_eq!(retained, reference);
}

#[test]
fn cli_static_output_and_errors() {
    use std::process::Command;
    let binary = env!("CARGO_BIN_EXE_kurage");
    let output = Command::new(binary)
        .args([
            "--static", "--ascii", "--width", "8", "--height", "5", "--time", "-0.1",
        ])
        .env("PATH", "")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_ascii());
    assert_eq!(String::from_utf8(output.stdout).unwrap().lines().count(), 5);
    assert!(
        !Command::new(binary)
            .args(["--fps", "9"])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        !Command::new(binary)
            .args(["--time", "NaN"])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        !Command::new(env!("CARGO_BIN_EXE_kurage-export"))
            .args(["output/unused.gif", "--workers", "0"])
            .output()
            .unwrap()
            .status
            .success()
    );
}

#[test]
fn source_test_inventory_is_complete() {
    let names = [
        "test_cached_glyphs_match_text_drawing",
        "test_closeup_detail_preserves_light_and_color",
        "test_heading_tracks_forward_motion",
        "test_loop_has_no_boundary_jump",
        "test_parallel_rendering_preserves_temporal_filter",
        "test_recycling_is_invisible",
        "test_shared_memory_cleanup_on_close_and_error",
        "test_standalone_modes_and_small_viewports",
    ];
    for name in names {
        assert!(include_str!("parity.rs").contains(&format!("fn {name}(")));
    }
    assert_eq!(
        names.as_slice(),
        array(&oracle()["source_tests"])
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>()
    );
}

#[cfg(unix)]
#[test]
fn failed_encoder_cleans_child_and_temporary_files() {
    use std::{os::unix::fs::PermissionsExt, process::Command};
    let root = tempfile::tempdir().unwrap();
    let bin = root.path().join("bin");
    let temporary = root.path().join("temporary");
    std::fs::create_dir(&bin).unwrap();
    std::fs::create_dir(&temporary).unwrap();
    let encoder = bin.join("ffmpeg");
    std::fs::write(&encoder, b"#!/bin/sh\nexit 17\n").unwrap();
    std::fs::set_permissions(&encoder, std::fs::Permissions::from_mode(0o755)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_kurage-export"))
        .arg(root.path().join("failed.gif"))
        .args(["--single", "--workers", "1"])
        .env("PATH", &bin)
        .env("TMPDIR", &temporary)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(std::fs::read_dir(&temporary).unwrap().count(), 0);
}
