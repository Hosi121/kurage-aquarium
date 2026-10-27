# Kurage Aquarium

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
off-screen recycling, loop continuity, and the standalone renderer.
