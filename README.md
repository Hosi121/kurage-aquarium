# Kurage Aquarium

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Latest tag](https://img.shields.io/github/v/tag/Hosi121/kurage-aquarium)](https://github.com/Hosi121/kurage-aquarium/tags)

Procedural jellyfish drawn with colored Braille dots. Preview a jellyfish in your
terminal, or export a looping aquarium GIF for a Windows Terminal background.

![Animated aquarium preview](assets/preview.gif)

The aquarium uses eight independent swimmers with staggered entrances and exits.
Each bell points along its path, contracts to advance, and leaves its filaments
following earlier positions and orientations. The motion is a visual
approximation, not a fluid simulation.

## Install

Python 3.10 or later is required. GIF export also requires FFmpeg and a font
containing Braille glyphs. Development and export have been tested on Ubuntu/WSL
with Python 3.12.

On Ubuntu/WSL, install the system dependencies:

```sh
sudo apt install python3-venv ffmpeg fonts-dejavu-core
```

Clone and install:

```sh
git clone https://github.com/Hosi121/kurage-aquarium.git
cd kurage-aquarium
python3 -m venv .venv
source .venv/bin/activate
python -m pip install -e .
```

## Terminal preview

```sh
kurage
kurage --ascii
kurage --static --width 52 --height 42
```

The live viewer uses 30 fps by default; press Ctrl+C to stop. Braille mode draws
2 × 4 dots per character cell. `--ascii` selects ordinary ASCII characters.
Redirected output is a still frame. `NO_COLOR` or `--no-color` disables ANSI colors.

## Export a background

```sh
kurage-export output/aquarium.gif
```

The default aquarium is 1280 × 720, 30 fps, and 38.4 seconds long. Rendering takes
time; it prints progress while FFmpeg writes the GIF. A PNG of the first frame is
saved alongside the GIF. Once exported, playback needs no Python process.

Export uses up to four CPU processes by default. To choose the process count:

```sh
kurage-export output/aquarium.gif --workers 4
kurage-export output/aquarium.gif --workers 1
```

Independent light buffers are rendered in parallel. The temporal filter still
runs in frame order, preserving the motion and colors. Only a bounded number of
frames are in flight; the whole animation is not kept in memory. Glyph masks are
cached instead of rasterizing the same characters again for each frame.

One complete default export on a WSL machine with 16 available logical CPUs
(Python 3.12, NumPy 2.5.4, Pillow 13.0.0) measured:

| Implementation | Rendering processes | Wall time |
| --- | ---: | ---: |
| Original exporter (`v0.1.0`) | 1 | 77.79 s |
| Parallel rendering and cached glyphs | 4 | 21.74 s |

These are single runs including startup, palette creation, rendering, and GIF
encoding, using the same 1280 × 720, 30 fps, 38.4-second scene and font. Both the
GIF and first-frame PNG had identical SHA-256 hashes before and after. Timings
depend on the machine; `--workers 1` is available for constrained environments.

The exporter looks for DejaVu Sans. To use another font with Braille glyphs:

```sh
kurage-export output/aquarium.gif --font /path/to/font.ttf
```

For the earlier single-jellyfish background:

```sh
kurage-export output/single.gif --single
```

### Windows Terminal

Copy the exported GIF to a Windows-accessible folder. In the desired profile in
Windows Terminal's `settings.json`, add these settings with your image's path:

```json
{
    "backgroundImage": "C:\\path\\to\\aquarium.gif",
    "backgroundImageOpacity": 0.7,
    "backgroundImageStretchMode": "uniformToFill",
    "backgroundImageAlignment": "center"
}
```

These keys are also available in [examples/windows-terminal.json](examples/windows-terminal.json).
`backgroundImageOpacity` controls the image's strength; it does not change the
window's overall transparency. The example does not install or change settings
automatically.

## How it works

- `renderer.py` builds the bell, oral arms, and filaments as 3D point samples,
  shades them, and converts the light buffer to colored Braille or ASCII.
- `aquarium.py` controls the swimmers' paths, speed, depth, and orientation.
  Propulsion changes progress along the same path that defines body heading.
  Filaments use delayed positions and body frames. Recycling happens beyond the
  viewport.
- `export.py` draws the glyphs with Pillow and streams RGB frames to FFmpeg,
  using a shared palette for consistent GIF colors.

The renderer and configuration live in source control. Full-size generated
backgrounds belong in `output/`, which is ignored by Git. The checked-in preview
is a smaller rendition of the complete loop.

## Checks

```sh
python -m unittest discover -s tests -v
```

The checks cover forward motion, heading alignment, body-frame geometry,
off-screen recycling, loop continuity, the standalone renderer, cached glyph
equivalence, and serial/parallel output with temporal filtering.

## License

MIT License. See [LICENSE](LICENSE) for the full text.
