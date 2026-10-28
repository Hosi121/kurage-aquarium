#!/usr/bin/env python3
"""Render a seamless, 30 fps Windows Terminal background from the jellyfish."""

import argparse
from collections import deque
from concurrent.futures import ProcessPoolExecutor
from contextlib import closing
from itertools import islice
import multiprocessing
from multiprocessing import shared_memory
from multiprocessing.util import Finalize
import os
from pathlib import Path
import subprocess
import tempfile
import shutil

import numpy as np
from PIL import Image, ImageDraw, ImageFont

from .renderer import BACKGROUND, BRAILLE, Jellyfish
from .aquarium import Aquarium, PERIOD as AQUARIUM_PERIOD


class GlyphPainter:
    """Reuse glyph coverage, preserving text blending and drawing order."""

    def __init__(self, font, cell_width, cell_height, offset_x=0, size=(1280, 720)):
        self.font = font
        self.cell_width, self.cell_height = cell_width, cell_height
        self.offset_x, self.size = offset_x, size
        self.masks = {}
        self.atlas = self._make_atlas()
        if self.atlas is not None:
            self.atlas_background = (255 - self.atlas[:, :, :, None]) * np.array(BACKGROUND, dtype=np.uint16)
            self.canvas = np.array(Image.new("RGB", self.size, BACKGROUND))

    def _mask(self, char):
        if char not in self.masks:
            left, top, right, bottom = self.font.getbbox(char)
            mask = Image.new("L", (right - left, bottom - top))
            ImageDraw.Draw(mask).text((-left, -top), char, font=self.font, fill=255)
            self.masks[char] = mask, left, top
        return self.masks[char]

    def _make_atlas(self):
        atlas = np.zeros((256, self.cell_height, self.cell_width), dtype=np.uint16)
        for bits, char in enumerate(BRAILLE[1:], 1):
            mask, left, top = self._mask(str(char))
            ink = mask.getbbox()
            if ink is not None and (left + ink[0] < 0 or top + ink[1] < 0
                                    or left + ink[2] > self.cell_width
                                    or top + ink[3] > self.cell_height):
                # Custom fonts may overlap adjacent cells; preserve draw order.
                return None
            cell = Image.new("L", (self.cell_width, self.cell_height))
            cell.paste(mask, (left, top))
            atlas[bits] = np.asarray(cell)
        return atlas

    def draw(self, glyphs, colors):
        rows = min(len(glyphs), self.size[1] // self.cell_height)
        columns = glyphs.shape[1]
        ys, xs = np.nonzero(glyphs[:rows] != " ")
        if (self.atlas is not None and self.offset_x >= 0
                and self.offset_x + columns * self.cell_width <= self.size[0]
                and glyphs.dtype.kind == "U" and glyphs.dtype.itemsize == 4):
            codes = glyphs[ys, xs].view(np.uint32)
            if np.all((codes >= 0x2801) & (codes <= 0x28ff)):
                bits = codes - 0x2800
                coverage = self.atlas[bits, :, :, None]
                ink = colors[ys, xs].astype(np.uint16)[:, None, None, :]
                # Match Pillow's 8-bit mask compositing exactly, in bulk.
                rgb = ((self.atlas_background[bits] + coverage * ink + 127) // 255).astype(np.uint8)
                pixels = self.canvas.copy()
                cells = pixels[:rows * self.cell_height,
                               self.offset_x:self.offset_x + columns * self.cell_width]
                cells = cells.reshape(rows, self.cell_height, columns, self.cell_width, 3).transpose(0, 2, 1, 3, 4)
                cells[ys, xs] = rgb
                return Image.fromarray(pixels)
        image = Image.new("RGB", self.size, BACKGROUND)
        for y, x in zip(ys, xs):
            char = str(glyphs[y, x])
            mask, left, top = self._mask(char)
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


def _init_worker(single, memory_name, shape):
    global _worker_model, _worker_memory, _worker_buffers
    _worker_model = Jellyfish(loop_period=19.2) if single else Aquarium()
    _worker_memory = shared_memory.SharedMemory(name=memory_name)
    _worker_buffers = np.ndarray(shape, dtype=np.float64, buffer=_worker_memory.buf)
    Finalize(None, _worker_memory.close, exitpriority=10)


def _worker_light_frame(columns, rows, elapsed, slot):
    _worker_buffers[slot] = _worker_model.light_frame(columns, rows, elapsed)
    return slot


def light_frames(model, columns, rows, times, workers, *, single=False):
    """Bound in-flight buffers and deliver independent frames in time order."""
    if workers == 1:
        for elapsed in times:
            yield model.light_frame(columns, rows, elapsed)
        return
    frame_shape = (rows * 4, columns * 2, 3)
    frame_bytes = int(np.prod(frame_shape)) * np.dtype(np.float64).itemsize
    slots = min(workers * 2, max(1, 32 * 1024 * 1024 // frame_bytes))
    shape = (slots, *frame_shape)
    memory = shared_memory.SharedMemory(create=True, size=slots * frame_bytes)
    pool = None
    try:
        buffers = np.ndarray(shape, dtype=np.float64, buffer=memory.buf)
        pool = ProcessPoolExecutor(max_workers=workers, mp_context=multiprocessing.get_context("spawn"),
                                   initializer=_init_worker, initargs=(single, memory.name, shape))
        times = iter(times)
        pending = deque((slot, pool.submit(_worker_light_frame, columns, rows, t, slot))
                        for slot, t in enumerate(islice(times, slots)))
        while pending:
            slot, future = pending.popleft()
            future.result()
            # The caller owns its array; a reused slot cannot alter retained
            # frames or temporal-filter history after this generator closes.
            yield buffers[slot].copy()
            for t in islice(times, 1):
                pending.append((slot, pool.submit(_worker_light_frame, columns, rows, t, slot)))
    finally:
        try:
            if pool is not None:
                pool.shutdown(wait=True, cancel_futures=True)
        finally:
            memory.close()
            memory.unlink()


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
