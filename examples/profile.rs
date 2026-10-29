//! Small serial CPU-stage sample; not an end-to-end export benchmark.
use kurage_aquarium::{
    export::Painter,
    render::{Frame, Shader},
    scene::{LightWorkspace, Scene},
};
use std::time::{Duration, Instant};

fn main() {
    let single = std::env::args().any(|arg| arg == "--single");
    let scene = if single {
        Scene::single(Some(19.2))
    } else {
        Scene::aquarium()
    };
    let (columns, rows, period) = if single {
        (120, 50, 19.2)
    } else {
        (160, 45, 38.4)
    };
    let painter = Painter::new(single, None).unwrap();
    let mut workspace = LightWorkspace::default();
    let mut pixels = Vec::new();
    let mut shader = Shader::default();
    let mut frame = Frame::default();
    let mut image = Vec::new();
    let mut elapsed = [Duration::ZERO; 3];
    for section in 0..3 {
        let start = section as f64 * period / 3.;
        for i in 0..20 {
            let t = start + (i as f64 - 4.) / 30.;
            let before = Instant::now();
            scene.light_frame_into(columns, rows, t, false, &mut workspace, &mut pixels);
            let middle = Instant::now();
            shader.shade_into(&mut pixels, columns, rows, t, false, true, &mut frame);
            let painted = Instant::now();
            painter.draw_into(&frame, &mut image);
            if i >= 4 {
                elapsed[0] += middle - before;
                elapsed[1] += painted - middle;
                elapsed[2] += Instant::now() - painted;
            }
            std::hint::black_box(&image);
        }
    }
    println!(
        "{}",
        serde_json::json!({"scene":if single {"single"} else {"aquarium"},"frames":48,
        "light_ms":elapsed[0].as_secs_f64()*1000.,"shade_ms":elapsed[1].as_secs_f64()*1000.,
        "paint_ms":elapsed[2].as_secs_f64()*1000.,"note":"serial stage sample; FFmpeg and pipeline waits excluded"})
    );
}
