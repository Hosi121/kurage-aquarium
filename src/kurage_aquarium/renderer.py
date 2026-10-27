#!/usr/bin/env python3
"""A translucent terminal jellyfish with fine Braille dots. Ctrl+C to close."""

import argparse
import math
import os
import shutil
import signal
import sys
import time

import numpy as np


BACKGROUND = (4, 9, 14)
RAMP = np.array(list(" .,:-=+*#"))
BRAILLE = np.array([" "] + [chr(0x2800 + bits) for bits in range(1, 256)])
DOT_BITS = np.array(((1, 8), (2, 16), (4, 32), (64, 128)), dtype=np.uint16)
# Ordered coverage gives translucent surfaces an even, stable dot pattern.
DITHER = np.array(((0, 2), (3, 1)))
for _ in range(2):
    DITHER = np.block([[4 * DITHER, 4 * DITHER + 2],
                       [4 * DITHER + 3, 4 * DITHER + 1]])
DITHER = 0.025 + 0.67 * (DITHER + 0.5) / 64
CYAN = np.array((36, 185, 205), dtype=float) / 255
ROSE = np.array((194, 132, 166), dtype=float) / 255
GOLD = np.array((226, 174, 113), dtype=float) / 255
VIOLET = np.array((124, 113, 184), dtype=float) / 255
TAU = math.tau
PERIOD = 4.8


def contraction(elapsed):
    """A short squeeze and a long, gentle recovery, both with smooth ends."""
    phase = np.mod(np.asarray(elapsed), PERIOD) / PERIOD
    squeezing = phase < 0.28
    u = np.where(squeezing, phase / 0.28, (phase - 0.28) / 0.72)
    smooth = u * u * u * (10 + u * (-15 + 6 * u))
    return np.where(squeezing, smooth, 1 - smooth)


class Swimming:
    """A periodic jet-and-drag response, evaluated without frame-rate drift."""

    def __init__(self):
        count = 1024
        dt = PERIOD / count
        phase = np.arange(count) * dt
        q = contraction(phase)
        squeeze_speed = np.maximum(0, (np.roll(q, -1) - np.roll(q, 1)) / (2 * dt))
        # A steady, weak current balances the upward jet over one cycle.
        force = 0.78 * (squeeze_speed.mean() - squeeze_speed)
        velocity = 0.0
        speeds = np.zeros(count)
        for cycle in range(7):
            for i in range(count):
                velocity += dt * (force[i] - 1.9 * velocity)
                if cycle == 6:
                    speeds[i] = velocity
        speeds -= speeds.mean()
        positions = np.concatenate(([0.0], np.cumsum(speeds * dt)))
        gain = 0.27 / np.ptp(positions)
        positions = (positions - (positions.max() + positions.min()) / 2) * gain
        self.times = np.linspace(0, PERIOD, count + 1)
        self.positions = positions
        self.speeds = np.concatenate((speeds, speeds[:1])) * gain

    def position(self, elapsed):
        return np.interp(np.mod(elapsed, PERIOD), self.times, self.positions)

    def speed(self, elapsed):
        return np.interp(np.mod(elapsed, PERIOD), self.times, self.speeds)


class Jellyfish:
    """Transparent surfaces and filaments, splatted between terminal cells."""

    def __init__(self, *, loop_period=None):
        self.loop_period = loop_period
        self.swimming = Swimming()
        self._temporal_pixels = None
        self._temporal_dots = None
        self._temporal_time = None
        # Stable samples make a fine mesh without frame-to-frame randomness.
        phi, theta = np.meshgrid(
            np.linspace(0.04, math.pi / 2, 37),
            np.linspace(0, TAU, 128, endpoint=False), indexing="ij",
        )
        self.u, self.a = phi.ravel(), theta.ravel()
        self.radius = np.sin(self.u)
        self.cap_x = self.radius * np.cos(self.a)
        self.cap_y = -0.82 * np.cos(self.u)
        self.cap_z = 0.76 * self.radius * np.sin(self.a)
        self.ribs = np.maximum(0, np.cos(self.a * 20 + self.u * 1.5)) ** 18
        self.edge = np.sin(self.u) ** 12
        self.cap_color = CYAN[None, :] * (0.80 + 0.20 * self.edge[:, None])
        self.rim_a = np.linspace(0, TAU, 560, endpoint=False)
        self.thread_a = np.repeat(np.linspace(0, TAU, 26, endpoint=False), 115)
        self.thread_s = np.tile(np.linspace(0, 1, 115), 26)
        self.thread_length = 2.45 + 0.85 * (0.5 + 0.5 * np.sin(self.thread_a * 2.3 + 1))
        s, u = np.meshgrid(np.linspace(0, 1, 170), np.linspace(-1, 1, 7), indexing="ij")
        self.arm_s, self.arm_u = s.ravel(), u.ravel()
        angle, layer = np.meshgrid(
            np.linspace(0, TAU, 130, endpoint=False), np.linspace(-1, 1, 7), indexing="ij",
        )
        self.core_a, self.core_layer = angle.ravel(), layer.ravel()

    def angle(self, elapsed, speed):
        if self.loop_period is not None:
            speed = max(1, round(speed * self.loop_period / TAU)) * TAU / self.loop_period
            elapsed = np.mod(elapsed, self.loop_period)
        return elapsed * speed

    def points(self, elapsed, *, carry_swim=True):
        squeeze = float(contraction(elapsed))
        height = 0.93 + 0.17 * squeeze
        rim_scale = 1.035 - 0.17 * squeeze
        rim_lift = -0.075 * squeeze
        body_position = float(self.swimming.position(elapsed))
        pieces, emissions = [], []

        def add(x, y, z, rgb, energy):
            xyz = np.column_stack((x, y, z))
            colors = np.broadcast_to(rgb, (len(xyz), 3))
            energy = np.broadcast_to(np.asarray(energy), (len(xyz),))
            pieces.append(xyz)
            emissions.append(colors * energy[:, None])

        # Edge light and radial ridges reveal the translucent bell's volume.
        fresnel = (1 - np.abs(np.sin(self.a) * np.sin(self.u))) ** 2
        weight = (0.032 + 0.095 * fresnel + 0.10 * self.ribs + 0.065 * self.edge)
        weight *= np.maximum(0.12, self.radius)
        # The rim tucks in more than the shoulders during a squeeze.
        profile = 1.035 - squeeze * (0.055 + 0.115 * self.radius ** 4)
        scallop = 1 + 0.023 * np.cos(20 * self.a) * self.edge
        add(self.cap_x * profile * scallop,
            self.cap_y * height + (0.035 * np.cos(20 * self.a) + rim_lift) * self.edge,
            self.cap_z * profile * scallop, self.cap_color, weight)
        a = self.rim_a
        rim_radius = rim_scale * (1 + 0.023 * np.cos(20 * a))
        add(rim_radius * np.cos(a), 0.035 * np.cos(20 * a) + rim_lift,
            0.76 * rim_radius * np.sin(a), CYAN, 0.17)

        # A muted four-lobed organ is visible through the blue membrane.
        a, layer = self.core_a, self.core_layer
        core_r = 0.20 + 0.090 * np.cos(4 * a) + 0.073 * layer
        blush = 0.22 * (0.5 + 0.5 * np.sin(a))
        core_color = GOLD * (1 - blush[:, None]) + ROSE * blush[:, None]
        add(core_r * np.cos(a), -0.26 * height + 0.045 * layer + 0.04 * np.sin(2 * a),
            0.65 * core_r * np.sin(a), core_color, 0.115 * (1 - 0.35 * layer ** 2))

        # Delayed waves make the frilled oral arms follow the bell's motion.
        s, u = self.arm_s, self.arm_u
        delay = 0.24 * s + 1.55 * s * s
        carried = self.swimming.position(elapsed - delay) - body_position if carry_swim else np.zeros_like(s)
        pulse = contraction(elapsed - delay)
        for arm in range(4):
            a = arm * math.pi / 2 + 0.35
            wave = 8.5 * s - self.angle(elapsed, 0.62) + a
            fold = 42 * s - self.angle(elapsed, 0.48) + a + 0.45 * pulse
            twist = 8 * s + a + self.angle(elapsed, 0.15)
            ribbon_width = (0.17 + 0.048 * np.sin(fold)) * (1 - 0.40 * s)
            lag = 0.065 * (np.sin(self.angle(elapsed - delay, 0.27)) - math.sin(self.angle(elapsed, 0.27)))
            axis_x = 0.13 * math.cos(a) * (1 + s) + 0.14 * np.sin(wave) * s + 0.12 * s * s + lag
            depth_wave = (8.5 * s + a) * 0.79 - self.angle(elapsed, 0.62 * 0.79)
            axis_z = 0.12 * math.sin(a) * (1 + s) + 0.11 * np.cos(depth_wave) * s
            x = axis_x + u * ribbon_width * np.cos(twist)
            z = axis_z + u * ribbon_width * np.sin(twist)
            y = 0.04 - 0.04 * pulse + s * (2.65 + arm * 0.14) + 0.025 * np.sin(fold + u * 2.2) + carried * s
            base = (1 - s[:, None]) * ROSE + s[:, None] * VIOLET
            warmth = 0.35 * np.exp(-s * 4.5)
            rgb = base * (1 - warmth[:, None]) + GOLD * warmth[:, None]
            energy = 0.040 + 0.080 * (0.5 + 0.5 * np.cos(fold + u * 3)) ** 5
            energy *= (1 - 0.55 * s) * (0.65 + 0.35 * np.abs(u))
            add(x, y, z, rgb, energy)

        a, s = self.thread_a, self.thread_s
        delay = 0.35 * s + 2.1 * s * s
        trailing_pulse = contraction(elapsed - delay)
        trailing_radius = (1.035 - 0.17 * trailing_pulse) * (1 - 0.20 * s)
        trailing_radius *= 1 + 0.023 * np.cos(20 * a) * (1 - s)
        wave = 7 * s + a * 2.5 - self.angle(elapsed, 0.40)
        x = np.cos(a) * trailing_radius + 0.11 * np.sin(wave) * s + 0.19 * s * s
        depth_wave = (7 * s + a * 2.5) * 0.8 - self.angle(elapsed, 0.40 * 0.8)
        z = 0.76 * np.sin(a) * trailing_radius + 0.10 * np.cos(depth_wave) * s
        carried = self.swimming.position(elapsed - delay) - body_position if carry_swim else np.zeros_like(s)
        y = s * self.thread_length + 0.035 * np.cos(20 * a) - 0.075 * trailing_pulse + carried * s
        shade = (0.5 + 0.5 * np.sin(a + 0.7))[:, None]
        rgb = CYAN * (1 - s[:, None] * 0.65) + VIOLET * s[:, None] * 0.65
        rgb *= 0.58 + 0.42 * shade
        energy = (0.075 + 0.035 * np.sin(12 * s + a) ** 2) * (1 - 0.72 * s)
        add(x, y, z, rgb, energy)
        return np.concatenate(pieces), np.concatenate(emissions)

    def raster(self, width, height, elapsed, cell_aspect, compact, energy_gain):
        xyz, emission = self.points(elapsed)
        yaw = 0.20 * math.sin(self.angle(elapsed, 0.17))
        cy, sy = math.cos(yaw), math.sin(yaw)
        x = xyz[:, 0] * cy + xyz[:, 2] * sy
        z = xyz[:, 2] * cy - xyz[:, 0] * sy
        y = xyz[:, 1] + self.swimming.position(elapsed)
        tilt = (1.24 if compact else 0.26) + 0.045 * math.sin(self.angle(elapsed, 0.21))
        ct, st = math.cos(tilt), math.sin(tilt)
        x, y = x * ct + y * st, y * ct - x * st
        y = y * 0.94 + z * 0.34
        perspective = 5 / (5 - z)
        x, y = x * perspective, y * perspective
        if compact:
            world_width, world_height, center_x, center_y = 4.7, 2.7, 1.0, 0.05
        else:
            world_width, world_height, center_x, center_y = 3.1, 4.6, 0.27, 1.16
        scale = 0.94 * min(width / world_width, height / (world_height * cell_aspect))
        x = (x - center_x) * scale + width * 0.5 + math.sin(self.angle(elapsed, 0.27)) * scale * 0.045
        y = (y - center_y) * scale * cell_aspect + height * 0.5
        emission *= (scale / 11.5) ** 1.6 * energy_gain
        emission *= (0.86 + 0.14 * np.clip(z + 0.5, 0, 1))[:, None]
        ix, iy = np.floor(x).astype(int), np.floor(y).astype(int)
        fx, fy = x - ix, y - iy
        pixels = np.zeros((height * width, 3))
        # Sub-cell coverage moves light continuously between character cells.
        for dx, dy, weight in (
            (0, 0, (1 - fx) * (1 - fy)), (1, 0, fx * (1 - fy)),
            (0, 1, (1 - fx) * fy), (1, 1, fx * fy),
        ):
            px, py = ix + dx, iy + dy
            valid = (px >= 0) & (px < width) & (py >= 0) & (py < height)
            indices = py[valid] * width + px[valid]
            for channel in range(3):
                pixels[:, channel] += np.bincount(
                    indices, weights=emission[valid, channel] * weight[valid], minlength=height * width,
                )
        return pixels.reshape(height, width, 3)

    def frame(self, width, height, elapsed, *, mode="braille", temporal=False):
        width, height = max(1, width), max(1, height)
        compact = height < 14 and width >= 30
        fine = mode != "ascii"
        pixels = self.raster(
            width * 2 if fine else width, height * 4 if fine else height,
            elapsed, 0.96 if fine else 0.48, compact,
            8 / 2 ** 1.6 if fine else 1,
        )
        return self.shade(pixels, width, height, elapsed, fine=fine, temporal=temporal)

    def shade(self, pixels, width, height, elapsed, *, fine=True, temporal=False):
        """Convert a shared light buffer into colored terminal dots."""
        if temporal:
            dt = elapsed - self._temporal_time if self._temporal_time is not None else 0
            if (self._temporal_pixels is not None and self._temporal_pixels.shape == pixels.shape
                    and 0 < dt < 0.25):
                blend = -math.expm1(-dt / 0.028)
                pixels = self._temporal_pixels + blend * (pixels - self._temporal_pixels)
            else:
                self._temporal_dots = None
            self._temporal_pixels = pixels
            self._temporal_time = elapsed
        strength = pixels.max(axis=2)
        brightness = 1 - np.exp(-strength * 0.27)
        glyphs = RAMP[np.minimum(len(RAMP) - 1, (brightness ** 0.75 * len(RAMP)).astype(int))]
        hue = pixels / np.maximum(strength[:, :, None], 0.0001)
        colors = hue * (44 + 180 * brightness[:, :, None] ** 0.45)
        colors = np.clip(np.rint(colors), 0, 255).astype(np.uint8)
        if fine:
            threshold = np.tile(DITHER, ((height * 4 + 7) // 8, (width * 2 + 7) // 8))[:height * 4, :width * 2]
            if temporal and self._temporal_dots is not None:
                # A small deadband prevents a faint point near its coverage
                # threshold from repeatedly blinking on and off.
                threshold = threshold + np.where(self._temporal_dots, -0.012, 0.012)
            active = brightness > threshold
            if temporal:
                self._temporal_dots = active
            dots = active.reshape(height, 4, width, 2).transpose(0, 2, 1, 3)
            bits = np.sum(dots * DOT_BITS, axis=(2, 3))
            glyphs = BRAILLE[bits]
            weights = (brightness.reshape(height, 4, width, 2).transpose(0, 2, 1, 3) + 0.03) * dots
            dot_colors = colors.reshape(height, 4, width, 2, 3).transpose(0, 2, 1, 3, 4)
            colors = np.rint(
                np.sum(dot_colors * weights[:, :, :, :, None], axis=(2, 3))
                / np.maximum(weights.sum(axis=(2, 3))[:, :, None], 0.0001)
            ).astype(np.uint8)
        return glyphs, colors


def encode(frame, color=True, previous=None):
    """Only repaint changed runs; never clear the screen between frames."""
    glyphs, colors = frame
    height, width = glyphs.shape
    pieces = ["\033[H"]
    old_glyphs, old_colors = previous if previous is not None else (None, None)
    for y in range(height):
        if old_glyphs is not None:
            changed = (glyphs[y] != old_glyphs[y]) | np.any(colors[y] != old_colors[y], axis=1)
            changed &= (glyphs[y] != " ") | (old_glyphs[y] != " ")
            positions = np.flatnonzero(changed)
            if not positions.size:
                continue
            first, last = int(positions[0]), int(positions[-1]) + 1
        else:
            first, last = 0, width
        pieces.append(f"\033[{y + 1};{first + 1}H")
        active = None
        for x in range(first, last):
            char = glyphs[y, x]
            if color and char != " ":
                rgb = tuple((int(c) // 4) * 4 for c in colors[y, x])
                if rgb != active:
                    pieces.append(f"\033[38;2;{rgb[0]};{rgb[1]};{rgb[2]}m")
                    active = rgb
            pieces.append(str(char))
    return "".join(pieces)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--static", action="store_true", help="print one frame and exit")
    parser.add_argument("--no-color", action="store_true", help="disable ANSI colors")
    parser.add_argument("--ascii", action="store_true", help="use ordinary ASCII instead of 2-by-4 Braille dots")
    parser.add_argument("--color", action="store_true", help="color static output even when redirected")
    parser.add_argument("--width", type=int, help="preview width in terminal columns")
    parser.add_argument("--height", type=int, help="preview height in terminal rows")
    parser.add_argument("--time", type=float, default=0, help="animation time for a static preview")
    parser.add_argument("--fps", type=int, default=30, choices=range(10, 61), metavar="10..60")
    args = parser.parse_args()
    colored = (sys.stdout.isatty() or args.color) and "NO_COLOR" not in os.environ and not args.no_color
    jellyfish = Jellyfish()
    mode = "ascii" if args.ascii else "braille"
    if args.static or not sys.stdout.isatty():
        frame = jellyfish.frame(args.width or 44, args.height or 34, args.time, mode=mode)
        if colored:
            sys.stdout.write(encode(frame) + "\033[0m\n")
        else:
            print("\n".join("".join(row) for row in frame[0]))
        return

    def stop(_signum, _frame):
        raise KeyboardInterrupt

    for signum in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, stop)
    start = time.monotonic()
    previous, size = None, None
    deadline = start
    try:
        sys.stdout.write("\033[?25l")
        if colored:
            sys.stdout.write(f"\033[48;2;{BACKGROUND[0]};{BACKGROUND[1]};{BACKGROUND[2]}m")
        while True:
            columns, rows = shutil.get_terminal_size((44, 34))
            width, height = max(1, columns - 1), max(1, rows)
            if size != (width, height):
                sys.stdout.write("\033[2J")
                previous, size = None, (width, height)
            now = time.monotonic()
            frame = jellyfish.frame(width, height, now - start, mode=mode, temporal=True)
            sys.stdout.write(encode(frame, colored, previous))
            sys.stdout.flush()
            previous = frame
            deadline += 1 / args.fps
            remaining = deadline - time.monotonic()
            if remaining > 0:
                time.sleep(remaining)
            else:
                deadline = time.monotonic()
    except KeyboardInterrupt:
        pass
    finally:
        sys.stdout.write("\033[0m\033[?25h")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
