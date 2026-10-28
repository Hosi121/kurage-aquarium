import unittest

import numpy as np

from kurage_aquarium.aquarium import Aquarium, DRIFTERS, PERIOD
from kurage_aquarium.renderer import Jellyfish


class SwimmingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.scene = Aquarium()

    def test_heading_tracks_forward_motion(self):
        times = np.linspace(0, PERIOD, 257)
        for index in range(len(DRIFTERS)):
            with self.subTest(swimmer=index):
                _, frames, velocity = self.scene.kinematics(index, times)
                forward = -frames[:, :, 1]
                alignment = np.sum(forward * velocity, axis=-1) / np.linalg.norm(velocity, axis=-1)
                self.assertGreater(alignment.min(), 0.999999)
                self.assertTrue(np.all(velocity[:, 1] < 0))
                np.testing.assert_allclose(np.linalg.det(frames), 1, atol=1e-12)
                np.testing.assert_allclose(
                    np.swapaxes(frames, -1, -2) @ frames,
                    np.broadcast_to(np.eye(3), frames.shape), atol=1e-12,
                )

    def test_recycling_is_invisible(self):
        for index, d in enumerate(DRIFTERS):
            with self.subTest(swimmer=index):
                travel = (3.65 + 1.30 * d.size) - (-4.20 - 5.20 * d.size)
                model = self.scene.models[index]

                def progress(t):
                    return (t / PERIOD + d.entry_phase
                            - .8 * d.size * model.swimming.position(t * d.speed + d.pulse_phase) / travel)

                low, high = 0., PERIOD
                for _ in range(40):
                    mid = (low + high) / 2
                    if progress(mid) < 1:
                        low = mid
                    else:
                        high = mid
                reset = (low + high) / 2
                for t in (reset - 1/30, reset, reset + 1/30):
                    x, y, _ = self.scene.projected(index, t, 320, 180)
                    self.assertFalse(np.any((x >= 0) & (x < 320) & (y >= 0) & (y < 180)))

    def test_loop_has_no_boundary_jump(self):
        start = self.scene.frame(80, 23, 0)
        end = self.scene.frame(80, 23, PERIOD)
        np.testing.assert_array_equal(start[0], end[0])
        np.testing.assert_array_equal(start[1], end[1])
        left = self.scene.frame(80, 23, PERIOD - 1/30)[1].astype(float)
        right = self.scene.frame(80, 23, 1/30)[1].astype(float)
        center = start[1].astype(float)
        self.assertLess(np.abs(left - center).mean(), max(3, 3 * np.abs(right - center).mean()))

    def test_standalone_modes_and_small_viewports(self):
        model = Jellyfish()
        for mode in ("ascii", "braille"):
            for width, height in ((1, 1), (8, 5), (38, 30), (80, 8)):
                with self.subTest(mode=mode, size=(width, height)):
                    glyphs, colors = model.frame(width, height, 1.6, mode=mode)
                    self.assertEqual(glyphs.shape, (height, width))
                    self.assertEqual(colors.shape, (height, width, 3))
                    self.assertTrue(np.isfinite(colors).all())
                    if mode == "ascii":
                        self.assertTrue("".join(glyphs.flat).isascii())
                    else:
                        self.assertTrue(all(c == " " or 0x2801 <= ord(c) <= 0x28ff for c in glyphs.flat))

    def test_closeup_detail_preserves_light_and_color(self):
        ordinary, closeup = Jellyfish(), Jellyfish(detail=2)
        for elapsed in (0, 1.6, 3.2):
            with self.subTest(elapsed=elapsed):
                _, reference = ordinary.points(elapsed)
                _, detailed = closeup.points(elapsed)
                self.assertGreater(len(detailed), 2 * len(reference))
                np.testing.assert_allclose(detailed.sum(axis=0), reference.sum(axis=0), rtol=.01)


if __name__ == "__main__":
    unittest.main()
