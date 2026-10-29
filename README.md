# Kurage Aquarium

Procedural jellyfish rendered as colored Braille dots. Use the native terminal
viewer, or generate a looping aquarium GIF for Windows Terminal and WezTerm.

![Animated aquarium preview](assets/preview.gif)

Nine independent swimmers follow gently curving paths. A giant foreground
jellyfish fills about 73% of the image width in the opening frame. Each bell
points along its motion; its filaments follow earlier positions and orientations.
Swimmers enter and leave outside the viewport. The motion is a visual
approximation of swimming.

## Install the native generator

Download the archive for your OS and CPU from the
[v0.4.1 release](https://github.com/Hosi121/kurage-aquarium/releases/tag/v0.4.1).
It contains `kurage` and `kurage-export` (`.exe` on Windows).

| OS | Archive target |
| --- | --- |
| Linux x86-64 | `x86_64-unknown-linux-gnu` |
| Windows x86-64 | `x86_64-pc-windows-msvc` |
| macOS Apple Silicon | `aarch64-apple-darwin` |
| macOS Intel | `x86_64-apple-darwin` |

Extract the archive and put the two executables in a directory in your PATH,
or run them using their full paths. On Linux/macOS you can try `./kurage` in
the extracted directory; on PowerShell use `.\kurage.exe`.

**No Python, NumPy, Pillow, or Rust installation is needed to run the binaries.**
GIF export requires `ffmpeg` in PATH. The default Braille glyph coverage and
scene palettes are embedded, so a font installation is no longer required.
FreeType is compiled in for optional custom fonts. Linux binaries are built on
Ubuntu 22.04; platform build/test details are in GitHub Actions.

## Use a pre-rendered background

If you only want the background, download `aquarium.gif` or
`kurage-wezterm.tar.gz` from the
[background release v0.3.0](https://github.com/Hosi121/kurage-aquarium/releases/tag/v0.3.0).
Playback needs no generator or FFmpeg. The full GIF is 21.1 MiB.

- **macOS / Ubuntu / Linux:** follow the [WezTerm setup guide](docs/wezterm.md).
- **Windows Terminal:** use the [profile settings example](examples/windows-terminal.json).

The background is centered at 70% opacity. WezTerm animates backgrounds while
the window is focused; see its [background reference](https://wezterm.org/config/lua/config/background.html).
GNOME Terminal and macOS Terminal do not use WezTerm's configuration.
Configuration loading was checked on Linux WezTerm; native macOS GUI playback
remains unverified. The Rust migration does not alter terminal input bindings.
For Codex wheel scrolling, see the
[scrollback troubleshooting steps](docs/wezterm.md#scrolling-in-codex-recalls-previous-prompts).

## Terminal viewer

```sh
kurage
kurage --ascii
kurage --static --width 52 --height 42
kurage --static --color --time 1.6
```

The live single-jellyfish viewer runs at 30 fps by default; Ctrl+C stops it.
Braille draws 2 × 4 dots per character cell. `--ascii` uses ordinary ASCII.
Redirected output is a still frame. `NO_COLOR` or `--no-color` disables ANSI
colors. `--fps` accepts 10–60 for the live viewer.

## Generate a background

```sh
kurage-export output/aquarium.gif
kurage-export output/aquarium.gif --workers 4
kurage-export output/single.gif --single
kurage-export output/custom.gif --font /path/to/font.ttf
```

The default aquarium is 1280 × 720, 30 fps, and 38.4 seconds long. `--single`
generates the earlier 19.2-second single-jellyfish scene. A first-frame PNG is
saved beside the GIF. Existing output files are overwritten.

Up to four render threads are used by default. Independent light buffers are
rendered in parallel while the preceding batch is shaded, painted, and sent to
FFmpeg. Temporal shading and GIF input remain in frame order. Two light-buffer
batches cycle between the renderer and consumer, with per-worker scratch,
shading, and RGB buffers reused across frames.
In-flight light buffers are bounded to 32 MiB, with no process shared memory
or Python interpreter workers. Repeated ribbon and filament calculations are
reused. FFmpeg encodes the RGB stream using the embedded scene-wide palette.
`--font` changes glyph rasterization while keeping that palette.

The default scene and glyph assets preserve the Python v0.3.0 output. Migration
checks, exact-output comparisons, benchmark commands, and compatibility details
are documented in [docs/rust-migration.md](docs/rust-migration.md).
The subsequent buffer-reuse and pipeline comparison is documented in
[docs/buffer-pipeline.md](docs/buffer-pipeline.md).

For optional lossless GIF compression with Gifsicle:

```sh
gifsicle -O3 -Okeep-empty output/aquarium.gif -o output/aquarium-optimized.gif
```

Compare file sizes before replacing the original. This extra pass is separate
from normal export.

## Build from source

Install [Rust](https://rust-lang.org/learn/get-started/) and a C++ build toolchain
for the bundled FreeType library (Ubuntu: `build-essential`; macOS: Xcode
Command Line Tools; Windows: Visual Studio C++ Build Tools).

```sh
git clone https://github.com/Hosi121/kurage-aquarium.git
cd kurage-aquarium
cargo build --locked --release --bins
./target/release/kurage
./target/release/kurage-export output/aquarium.gif
```

On Ubuntu, install FFmpeg separately with `sudo apt install ffmpeg`.
The generator is entirely Rust; its build does not invoke Python. Python is
only used by optional historical-oracle and release-packaging tools.

## Checks

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

Tests compare generated Python fixtures for geometry, body heading, forward
motion, off-screen recycling, loop continuity, temporal color and Braille/ASCII
output, ANSI output, full-resolution image hashes, custom-font clipping, and
parallel frame ownership. No Python is needed to run these tests.

## License

MIT. See [LICENSE](LICENSE) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)
for embedded glyph and FreeType notices.
