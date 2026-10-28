#!/usr/bin/env python3
"""Render a seamless, 30 fps Windows Terminal background from the jellyfish."""

import argparse
from collections import deque
from concurrent.futures import ProcessPoolExecutor
from contextlib import closing
from itertools import islice
import multiprocessing
import os
from pathlib import Path
import subprocess
import tempfile
import shutil

import numpy as np
from PIL import Image, ImageDraw, ImageFont

from .renderer import BACKGROUND, Jellyfish
from .aquarium import Aquarium, PERIOD as AQUARIUM_PERIOD


class GlyphPainter:
    """Reuse glyph coverage, preserving text blending and drawing order."""

    def __init__(self, font, cell_width, cell_height, offset_x=0, size=(1280, 720)):
        self.font = font
        self.cell_width, self.cell_height = cell_width, cell_height
        self.offset_x, self.size = offset_x, size
        self.masks = {}

    def draw(self, glyphs, colors):
        image = Image.new("RGB", self.size, BACKGROUND)
        rows = min(len(glyphs), self.size[1] // self.cell_height)
        ys, xs = np.nonzero(glyphs[:rows] != " ")
        for y, x in zip(ys, xs):
            char = str(glyphs[y, x])
            if char not in self.masks:
                left, top, right, bottom = self.font.getbbox(char)
                mask = Image.new("L", (right - left, bottom - top))
                ImageDraw.Draw(mask).text((-left, -top), char, font=self.font, fill=255)
                self.masks[char] = mask, left, top
            mask, left, top = self.masks[char]
            position = (self.offset_x + int(x) * self.cell_width + left,
                        int(y) * self.cell_height + top)
            image.paste(tuple(int(c) for c in colors[y, x]), position, mask)
        return image


def default_workers():
    try:
        available = len(os.sched_getaffinity(0))
    except (AttributeError, OSError):
        available = os.cpu_count() or 1
    return min(4, available)


def _init_worker(single):
    global _worker_model
    _worker_model = Jellyfish(loop_period=19.2) if single else Aquarium()


def _worker_light_frame(columns, rows, elapsed):
    return _worker_model.light_frame(columns, rows, elapsed)


def light_frames(model, columns, rows, times, workers, *, single=False):
    """Bound in-flight buffers and deliver independent frames in time order."""
    if workers == 1:
        for elapsed in times:
            yield model.light_frame(columns, rows, elapsed)
        return
    pool = ProcessPoolExecutor(max_workers=workers, mp_context=multiprocessing.get_context("spawn"),
                               initializer=_init_worker, initargs=(single,))
    try:
        times = iter(times)
        pending = deque(pool.submit(_worker_light_frame, columns, rows, t)
                        for t in islice(times, workers * 2))
        while pending:
            yield pending.popleft().result()
            for t in islice(times, 1):
                pending.append(pool.submit(_worker_light_frame, columns, rows, t))
    finally:
        pool.shutdown(wait=True, cancel_futures=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--font", type=Path, help="path to a TrueType font with Braille glyphs")
    parser.add_argument("--single", action="store_true", help="export the previous single-jellyfish scene")
    parser.add_argument("--workers", type=int, default=default_workers(),
                        help="render processes (default: up to 4 available CPUs; use 1 for serial export)")
    args = parser.parse_args()
    if args.workers < 1:
        parser.error("--workers must be at least 1")
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
    painter = GlyphPainter(font, cell_width, cell_height, offset_x, (width, height))
    shader = model.shader if isinstance(model, Aquarium) else model

    def image_at(t, temporal=False):
        glyphs, colors = model.frame(columns, rows, t, temporal=temporal)
        return painter.draw(glyphs, colors)

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
            count = round(period * 30)
            times = (i / 30 for i in range(count))
            with closing(light_frames(model, columns, rows, times, args.workers, single=args.single)) as frames:
                for i, pixels in enumerate(frames):
                    glyphs, colors = shader.shade(pixels, columns, rows, i / 30, temporal=True)
                    image = painter.draw(glyphs, colors)
                    if i == 0:
                        image.save(args.output.with_suffix(".png"))
                    process.stdin.write(image.tobytes())
                    if i % 120 == 0:
                        print(f"Rendered {i}/{count} frames", flush=True)
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
