use crate::{
    BACKGROUND,
    export::Result,
    render::{Frame, Shader, encode},
    scene::Scene,
};
use clap::Parser;
use std::{
    io::{self, IsTerminal, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Parser)]
#[command(
    version,
    about = "A translucent terminal jellyfish with fine Braille dots. Ctrl+C to close."
)]
pub struct Viewer {
    #[arg(long = "static")]
    pub still: bool,
    #[arg(long)]
    pub no_color: bool,
    #[arg(long)]
    pub ascii: bool,
    #[arg(long)]
    pub color: bool,
    #[arg(long)]
    pub width: Option<usize>,
    #[arg(long)]
    pub height: Option<usize>,
    #[arg(long, default_value_t = 0., allow_hyphen_values = true)]
    pub time: f64,
    #[arg(long,default_value_t=30,value_parser=clap::value_parser!(u32).range(10..=60))]
    pub fps: u32,
}

struct Cursor;
impl Drop for Cursor {
    fn drop(&mut self) {
        let mut out = io::stdout();
        let _ = out.write_all(b"\x1b[0m\x1b[?25h");
        let _ = out.flush();
    }
}

pub fn view(args: Viewer) -> Result<()> {
    if !args.time.is_finite() {
        return Err("--time must be finite".into());
    }
    let terminal = io::stdout().is_terminal();
    let colored =
        (terminal || args.color) && std::env::var_os("NO_COLOR").is_none() && !args.no_color;
    let scene = Scene::single(None);
    let mut shader = Shader::default();
    if args.still || !terminal {
        let (width, height) = (
            args.width.unwrap_or(44).max(1),
            args.height.unwrap_or(34).max(1),
        );
        let frame = shader.shade(
            scene.light_frame(width, height, args.time, args.ascii),
            width,
            height,
            args.time,
            args.ascii,
            false,
        );
        if colored {
            println!("{}\x1b[0m", encode(&frame, true, None));
        } else {
            for row in frame.glyphs.chunks(width) {
                println!("{}", row.iter().collect::<String>());
            }
        }
        return Ok(());
    }
    let running = Arc::new(AtomicBool::new(true));
    let signal = Arc::clone(&running);
    ctrlc::set_handler(move || signal.store(false, Ordering::Relaxed))?;
    let _cursor = Cursor;
    let start = Instant::now();
    let mut deadline = start;
    let mut previous: Option<Frame> = None;
    let mut size = None;
    let mut output = io::stdout();
    write!(output, "\x1b[?25l")?;
    if colored {
        write!(
            output,
            "\x1b[48;2;{};{};{}m",
            BACKGROUND[0], BACKGROUND[1], BACKGROUND[2]
        )?;
    }
    while running.load(Ordering::Relaxed) {
        let (columns, rows) = crossterm::terminal::size().unwrap_or((44, 34));
        let (width, height) = (
            (columns as usize).saturating_sub(1).max(1),
            (rows as usize).max(1),
        );
        if size != Some((width, height)) {
            write!(output, "\x1b[2J")?;
            previous = None;
            size = Some((width, height));
        }
        let t = start.elapsed().as_secs_f64();
        let frame = shader.shade(
            scene.light_frame(width, height, t, args.ascii),
            width,
            height,
            t,
            args.ascii,
            true,
        );
        output.write_all(encode(&frame, colored, previous.as_ref()).as_bytes())?;
        output.flush()?;
        previous = Some(frame);
        deadline += Duration::from_secs_f64(1. / args.fps as f64);
        if let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
            std::thread::sleep(remaining);
        } else {
            deadline = Instant::now();
        }
    }
    Ok(())
}
