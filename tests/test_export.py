from contextlib import closing
import unittest

import numpy as np
from PIL import Image, ImageDraw, ImageFont

from kurage_aquarium.aquarium import Aquarium
from kurage_aquarium.export import GlyphPainter, light_frames
from kurage_aquarium.renderer import BACKGROUND, BRAILLE, Jellyfish


class ExportTests(unittest.TestCase):
    def test_cached_glyphs_match_text_drawing(self):
        glyphs = np.tile(BRAILLE.reshape(16, 16), (1, 2))
        colors = np.random.default_rng(0).integers(0, 256, (*glyphs.shape, 3), dtype=np.uint8)
        for font_size, cell_width, cell_height in ((12, 8, 16), (14, 10, 20)):
            with self.subTest(font_size=font_size):
                try:
                    font = ImageFont.truetype("DejaVuSans.ttf", font_size)
                except OSError:
                    self.skipTest("DejaVu Sans is not installed")
                painter = GlyphPainter(font, cell_width, cell_height, offset_x=-1)
                expected = Image.new("RGB", painter.size, BACKGROUND)
                draw = ImageDraw.Draw(expected)
                for y, row in enumerate(glyphs):
                    for x, char in enumerate(row):
                        if char != " ":
                            draw.text((-1 + x * cell_width, y * cell_height), str(char), font=font,
                                      fill=tuple(int(c) for c in colors[y, x]))
                # Include overlapping glyphs, image-edge clipping, and a reused cache.
                for _ in range(2):
                    self.assertEqual(painter.draw(glyphs, colors).tobytes(), expected.tobytes())

    def test_parallel_rendering_preserves_temporal_filter(self):
        times = [-1/30, 0, 1/30, 2/30, 38.4, 38.4 + 1/30]
        for single in (False, True):
            for workers in (1, 2):
                with self.subTest(single=single, workers=workers):
                    reference = Jellyfish(loop_period=19.2) if single else Aquarium()
                    model = Jellyfish(loop_period=19.2) if single else Aquarium()
                    shader = model if single else model.shader
                    with closing(light_frames(model, 38, 22, times, workers, single=single)) as frames:
                        buffers = list(frames)
                    self.assertEqual(len(buffers), len(times))
                    for elapsed, pixels in zip(times, buffers):
                        expected = reference.frame(38, 22, elapsed, temporal=True)
                        actual = shader.shade(pixels, 38, 22, elapsed, temporal=True)
                        np.testing.assert_array_equal(actual[0], expected[0])
                        np.testing.assert_array_equal(actual[1], expected[1])


if __name__ == "__main__":
    unittest.main()
