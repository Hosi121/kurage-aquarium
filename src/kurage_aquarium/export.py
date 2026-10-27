#!/usr/bin/env python3
"""Render a seamless, 30 fps Windows Terminal background from the jellyfish."""

import argparse
from pathlib import Path
import subprocess
import tempfile
import shutil

import numpy as np
from PIL import Image, ImageDraw, ImageFont

from .renderer import BACKGROUND, Jellyfish
from .aquarium import Aquarium, PERIOD as AQUARIUM_PERIOD


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--font", type=Path, help="path to a TrueType font with Braille glyphs")
    parser.add_argument("--single", action="store_true", help="export the previous single-jellyfish scene")
    args = parser.parse_args()
    if shutil.which("ffmpeg") is None:
        parser.error("ffmpeg is required to export GIFs; install it and make it available in PATH")
    if args.single:
        period = 19.2
        model = Jellyfish(loop_period=period)
        columns, rows, cell_width, cell_height, offset_x = 120, 50, 10, 20, 40
        font_size = 14
    else:
        period = AQUARIUM_PERIOD
        model = Aquarium()
        columns, rows, cell_width, cell_height, offset_x = 160, 45, 8, 16, 0
        font_size = 12
    try:
        font = ImageFont.truetype(str(args.font) if args.font else "DejaVuSans.ttf", font_size)
    except OSError:
        parser.error("a Braille-capable font is required; install DejaVu Sans or pass --font /path/to/font.ttf")
    width, height = 1280, 720

    def image_at(t, temporal=False):
        glyphs, colors = model.frame(columns, rows, t, temporal=temporal)
        image = Image.new("RGB", (width, height), BACKGROUND)
        draw = ImageDraw.Draw(image)
        for y in range(min(rows, height // cell_height)):
            for x in range(columns):
                if glyphs[y, x] != " ":
                    draw.text((offset_x + x * cell_width, y * cell_height), str(glyphs[y, x]),
                              font=font, fill=tuple(int(c) for c in colors[y, x]))
        return image

    # Build one palette for the entire animation, keeping colors stable at
    # frame boundaries. ffmpeg encodes incrementally, without retaining every
    # full-resolution RGB frame in memory.
    samples = Image.new("RGB", (width * 3, height * 2))
    for i in range(6):
        samples.paste(image_at(i * period / 6), (i % 3 * width, i // 3 * height))
    palette = samples.quantize(colors=256)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    first, last = model.frame(columns, rows, 0), model.frame(columns, rows, period)
    assert np.array_equal(first[0], last[0]) and np.array_equal(first[1], last[1]), "Loop mismatch"
    with tempfile.TemporaryDirectory(prefix="kurage-export-") as temporary:
        palette_path = Path(temporary) / "palette.png"
        palette_image = Image.new("P", (16, 16))
        palette_image.putpalette(palette.getpalette())
        palette_image.putdata(range(256))
        palette_image.convert("RGB").save(palette_path)
        command = [
            "ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
            "-f", "rawvideo", "-pixel_format", "rgb24", "-video_size", f"{width}x{height}",
            "-framerate", "30", "-i", "pipe:0", "-i", str(palette_path),
            "-filter_complex", "[0:v][1:v]paletteuse=dither=none:diff_mode=rectangle",
            "-loop", "0", str(args.output),
        ]
        # Warm up the tiny temporal filter before the first exported frame.
        for i in range(-18, 0):
            model.frame(columns, rows, i / 30, temporal=True)
        process = subprocess.Popen(command, stdin=subprocess.PIPE)
        try:
            for i in range(round(period * 30)):
                image = image_at(i / 30, temporal=True)
                if i == 0:
                    image.save(args.output.with_suffix(".png"))
                process.stdin.write(image.tobytes())
                if i % 120 == 0:
                    print(f"Rendered {i}/{round(period * 30)} frames", flush=True)
            process.stdin.close()
            if process.wait() != 0:
                raise RuntimeError("Background encoding failed")
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait()
    print(f"Saved {args.output} ({args.output.stat().st_size / 1024 / 1024:.1f} MiB)", flush=True)


if __name__ == "__main__":
    main()
