use crate::{
    BACKGROUND,
    pipeline::{self, RenderConfig},
    render::{Frame, Shader},
    scene::{LightWorkspace, Scene},
};
use freetype::{Library, face::LoadFlag};
use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

struct Glyph {
    left: i32,
    top: i32,
    width: usize,
    height: usize,
    mask: Vec<u8>,
}

struct Ink {
    offset: u32,
    alpha: u16,
    background: [u16; 3],
}
#[derive(Default)]
struct CellInk {
    opaque: Vec<u32>,
    translucent: Vec<Ink>,
}
pub struct Painter {
    glyphs: Vec<Glyph>,
    cell_width: usize,
    cell_height: usize,
    offset: i32,
    fixed_ink: Option<Vec<CellInk>>,
}

impl Painter {
    pub fn new(single: bool, font: Option<&Path>) -> Result<Self> {
        let (cw, ch, size, offset) = if single {
            (10, 20, 14, 40)
        } else {
            (8, 16, 12, 0)
        };
        Self::with_font(cw, ch, size, offset, font)
    }
    pub fn with_font(
        cw: usize,
        ch: usize,
        size: u32,
        offset: i32,
        font: Option<&Path>,
    ) -> Result<Self> {
        let mut glyphs = Vec::with_capacity(256);
        if let Some(path) = font {
            let library = Library::init()?;
            // Read through Rust so non-ASCII font paths work on Windows too.
            let face = library.new_memory_face(std::fs::read(path)?, 0)?;
            face.set_pixel_sizes(0, size)?;
            if face.get_char_index(0x2801).is_none() {
                return Err("font has no Braille glyphs".into());
            }
            let ascender = face
                .size_metrics()
                .ok_or("font has no size metrics")?
                .ascender
                / 64;
            for bits in 0..256 {
                face.load_char(if bits == 0 { 32 } else { 0x2800 + bits }, LoadFlag::RENDER)?;
                let slot = face.glyph();
                let bitmap = slot.bitmap();
                let width = bitmap.width() as usize;
                let height = bitmap.rows() as usize;
                let stride = bitmap.pitch().unsigned_abs() as usize;
                let mut mask = vec![0; width * height];
                for y in 0..height {
                    let source = if bitmap.pitch() < 0 {
                        height - 1 - y
                    } else {
                        y
                    };
                    mask[y * width..(y + 1) * width].copy_from_slice(
                        &bitmap.buffer()[source * stride..source * stride + width],
                    );
                }
                glyphs.push(Glyph {
                    left: slot.bitmap_left(),
                    top: ascender as i32 - slot.bitmap_top(),
                    width,
                    height,
                    mask,
                });
            }
        } else {
            let atlas: &[u8] = match (cw, ch, size) {
                (8, 16, 12) => include_bytes!("../assets/render/aquarium-glyphs.bin"),
                (10, 20, 14) => include_bytes!("../assets/render/single-glyphs.bin"),
                _ => return Err("custom cell sizes require a font".into()),
            };
            for bits in 0..256 {
                glyphs.push(Glyph {
                    left: 0,
                    top: 0,
                    width: cw,
                    height: ch,
                    mask: atlas[bits * cw * ch..(bits + 1) * cw * ch].to_vec(),
                });
            }
        }
        let fixed_ink = font.is_none().then(|| {
            glyphs
                .iter()
                .map(|glyph| {
                    let mut cell = CellInk::default();
                    for (i, &alpha) in glyph.mask.iter().enumerate() {
                        if alpha == 0 {
                            continue;
                        }
                        let offset = ((i / cw * 1280 + i % cw) * 3) as u32;
                        if alpha == 255 {
                            cell.opaque.push(offset);
                        } else {
                            cell.translucent.push(Ink {
                                offset,
                                alpha: alpha as u16,
                                background: BACKGROUND
                                    .map(|v| ((255 - alpha as u16) * v as u16) + 127),
                            });
                        }
                    }
                    cell
                })
                .collect()
        });
        Ok(Self {
            glyphs,
            cell_width: cw,
            cell_height: ch,
            offset,
            fixed_ink,
        })
    }

    pub fn draw(&self, frame: &Frame) -> Vec<u8> {
        let mut image = Vec::new();
        self.draw_into(frame, &mut image);
        image
    }

    pub fn draw_into(&self, frame: &Frame, image: &mut Vec<u8>) {
        image.resize(1280 * 720 * 3, 0);
        for p in image.chunks_exact_mut(3) {
            p.copy_from_slice(&BACKGROUND);
        }
        let rows = frame.height.min(720 / self.cell_height);
        if let Some(ink) = &self.fixed_ink {
            let fits = self.offset >= 0
                && self.offset as usize <= 1280
                && frame.width <= (1280 - self.offset as usize) / self.cell_width;
            if fits {
                for y in 0..rows {
                    let row = (y * self.cell_height * 1280 + self.offset as usize) * 3;
                    for x in 0..frame.width {
                        let i = y * frame.width + x;
                        let code = frame.glyphs[i] as u32;
                        if code == 32 {
                            continue;
                        }
                        let cell = &ink[(code - 0x2800) as usize];
                        let base = row + x * self.cell_width * 3;
                        let rgb = frame.colors[i];
                        for &offset in &cell.opaque {
                            let dest = base + offset as usize;
                            image[dest..dest + 3].copy_from_slice(&rgb);
                        }
                        for pixel in &cell.translucent {
                            let dest = base + pixel.offset as usize;
                            for j in 0..3 {
                                image[dest + j] = ((pixel.background[j] as u32
                                    + pixel.alpha as u32 * rgb[j] as u32)
                                    / 255) as u8;
                            }
                        }
                    }
                }
                return;
            }
        }
        for y in 0..rows {
            for x in 0..frame.width {
                let i = y * frame.width + x;
                let code = frame.glyphs[i] as u32;
                if code == 32 {
                    continue;
                }
                let glyph = &self.glyphs[(code - 0x2800) as usize];
                for gy in 0..glyph.height {
                    let py = y as i32 * self.cell_height as i32 + glyph.top + gy as i32;
                    if !(0..720).contains(&py) {
                        continue;
                    }
                    for gx in 0..glyph.width {
                        let px = self.offset
                            + x as i32 * self.cell_width as i32
                            + glyph.left
                            + gx as i32;
                        if !(0..1280).contains(&px) {
                            continue;
                        }
                        let alpha = glyph.mask[gy * glyph.width + gx] as u32;
                        if alpha == 0 {
                            continue;
                        }
                        let dest = (py as usize * 1280 + px as usize) * 3;
                        for j in 0..3 {
                            // Pillow blends over the existing pixel, including overlapping glyphs.
                            image[dest + j] = (((255 - alpha) * image[dest + j] as u32
                                + alpha * frame.colors[i][j] as u32
                                + 127)
                                / 255) as u8;
                        }
                    }
                }
            }
        }
    }
}

pub fn write_png(path: &Path, width: u32, height: u32, pixels: &[u8]) -> Result<()> {
    let file = BufWriter::new(File::create(path)?);
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(pixels)?;
    writer.finish()?;
    Ok(())
}

struct Encoder(Child);
impl Drop for Encoder {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub fn default_workers() -> usize {
    std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(4)
}

pub fn export(output: PathBuf, single: bool, workers: usize, font: Option<PathBuf>) -> Result<()> {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let running = Arc::new(AtomicBool::new(true));
    let signal = Arc::clone(&running);
    ctrlc::set_handler(move || signal.store(false, Ordering::Relaxed))?;
    if workers == 0 {
        return Err("--workers must be at least 1".into());
    }
    if output
        .extension()
        .and_then(|s| s.to_str())
        .is_none_or(|s| !s.eq_ignore_ascii_case("gif"))
    {
        return Err("output must have a .gif extension".into());
    }
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
    let painter = Painter::new(single, font.as_deref())?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()?;
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let temp = tempfile::tempdir()?;
    let palette = temp.path().join("palette.png");
    let colors: &[u8] = if single {
        include_bytes!("../assets/render/single-palette.bin")
    } else {
        include_bytes!("../assets/render/aquarium-palette.bin")
    };
    write_png(&palette, 16, 16, colors)?;
    let child = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "rawvideo",
            "-pixel_format",
            "rgb24",
            "-video_size",
            "1280x720",
            "-framerate",
            "30",
            "-i",
            "pipe:0",
            "-i",
        ])
        .arg(&palette)
        .args([
            "-filter_complex",
            "[0:v][1:v]paletteuse=dither=none:diff_mode=rectangle",
            "-loop",
            "0",
        ])
        .arg(&output)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot start ffmpeg; install FFmpeg and put it in PATH: {e}"))?;
    let mut encoder = Encoder(child);
    let mut input = BufWriter::new(encoder.0.stdin.take().ok_or("ffmpeg stdin unavailable")?);
    let mut shader = Shader::default();
    let mut warmup = LightWorkspace::default();
    let mut light = Vec::new();
    let mut frame = Frame::default();
    let mut image = Vec::new();
    for i in -18..0 {
        let t = i as f64 / 30.;
        scene.light_frame_into(columns, rows, t, false, &mut warmup, &mut light);
        shader.shade_into(&mut light, columns, rows, t, false, true, &mut frame);
    }
    let count = (period * 30_f64).round() as usize;
    pipeline::for_each_frame(
        &scene,
        &pool,
        RenderConfig {
            columns,
            rows,
            count,
        },
        &running,
        (warmup, light),
        |i, pixels| {
            shader.shade_into(
                pixels,
                columns,
                rows,
                i as f64 / 30.,
                false,
                true,
                &mut frame,
            );
            painter.draw_into(&frame, &mut image);
            if i == 0 {
                write_png(&output.with_extension("png"), 1280, 720, &image)?;
            }
            input.write_all(&image)?;
            if i % 120 == 0 {
                println!("Rendered {i}/{count} frames");
            }
            Ok(())
        },
    )?;
    input.flush()?;
    drop(input);
    if !encoder.0.wait()?.success() {
        return Err("background encoding failed".into());
    }
    println!(
        "Saved {} ({:.1} MiB)",
        output.display(),
        std::fs::metadata(&output)?.len() as f64 / 1048576.
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_cell_painter_matches_generic_blending_and_clipped_fallback() {
        for single in [false, true] {
            for offset in [-1, 0, 40, 1278] {
                let (cw, ch, size) = if single { (10, 20, 14) } else { (8, 16, 12) };
                let fast = Painter::with_font(cw, ch, size, offset, None).unwrap();
                let mut generic = Painter::with_font(cw, ch, size, offset, None).unwrap();
                generic.fixed_ink = None;
                let mut frame = Frame {
                    width: 32,
                    height: 16,
                    glyphs: Vec::new(),
                    colors: Vec::new(),
                };
                for i in 0..512 {
                    let bits = i % 256;
                    frame.glyphs.push(if bits == 0 {
                        ' '
                    } else {
                        char::from_u32(0x2800 + bits as u32).unwrap()
                    });
                    frame
                        .colors
                        .push(std::array::from_fn(|j| ((i * 37 + j * 97) % 256) as u8));
                }
                assert_eq!(fast.draw(&frame), generic.draw(&frame));
            }
        }
    }
}
