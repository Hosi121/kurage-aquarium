# Kurage Aquarium native commands

These executables do not require Python, NumPy, Pillow, or a Rust installation.
The default Braille glyphs and scene palettes are embedded. GIF export requires
FFmpeg in PATH; playing a generated background does not require FFmpeg.

Put both executables in a directory in PATH, or run them from this directory:

```sh
./kurage
./kurage --ascii
./kurage --static --width 52 --height 42
./kurage-export output/aquarium.gif
./kurage-export output/single.gif --single
```

On Windows PowerShell, use `.\kurage.exe` and `.\kurage-export.exe` instead.
Ctrl+C stops the live viewer. `NO_COLOR` disables ANSI colors. GIF export uses
up to four threads by default; `--workers 1` selects one thread. Use `--help`
for all options.

The aquarium is 1280×720 at 30 fps with a seamless 38.4-second loop. The single
jellyfish preset has a 19.2-second loop. A first-frame PNG is saved beside the
GIF. Existing output files are overwritten. Default font installation is not
needed; `--font /path/to/font.ttf` selects a custom Braille-capable font.

Linux x86-64 binaries use glibc and are built on Ubuntu 22.04. Choose the archive
matching your OS and CPU. The Windows binary includes its C runtime.

Setup and terminal background documentation:
https://github.com/Hosi121/kurage-aquarium#readme

WezTerm setup and Codex mouse-wheel troubleshooting:
https://github.com/Hosi121/kurage-aquarium/blob/main/docs/wezterm.md

MIT licensed; see LICENSE and THIRD_PARTY_NOTICES.md. Third-party notices for
the embedded glyph coverage and compiled FreeType are included in `licenses/`.
