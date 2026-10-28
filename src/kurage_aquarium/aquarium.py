"""Jellyfish propulsion, heading and trailing anatomy follow the same path."""

from dataclasses import dataclass
import math

import numpy as np


from .renderer import BACKGROUND, Jellyfish
PERIOD = 38.4


@dataclass(frozen=True)
class Drifter:
    lane: float
    depth: float
    size: float
    speed: float
    pulse_phase: float
    entry_phase: float
    sway: float
    curve_phase: float
    lean: float
    spin: float
    tint: tuple


# Staggered passes keep several animals visible while others enter or leave
# beyond the viewport. No animal needs to reverse or teleport on screen.
DRIFTERS = (
    Drifter(-2.75, .65, .85, 1.0, .35, .22, .95, .10, -2.20, .10, (1.00, 1.00, 1.00)),
    Drifter(2.65, .25, .68, 7/8, 2.45, .345, .90, 2.20, 2.20, 1.15, (.96, .98, 1.08)),
    Drifter(-.35, -.85, .48, 9/8, 1.15, .470, .53, 4.10, -.70, -.85, (1.09, .94, 1.10)),
    Drifter(1.15, -1.45, .35, 3/4, 3.80, .595, .45, 1.20, .80, 2.00, (.92, 1.05, 1.08)),
    Drifter(-3.60, -.65, .43, 5/4, 2.80, .720, .53, 5.20, -.90, -1.40, (1.10, .93, 1.12)),
    Drifter(.75, .15, .54, 1.0, 4.10, .845, .88, 3.30, 1.40, .65, (.98, 1.02, 1.05)),
    Drifter(-1.45, -1.55, .32, 7/8, 1.80, .970, .40, 2.80, .30, -1.80, (.94, 1.04, 1.08)),
    Drifter(3.55, -1.05, .39, 9/8, .75, .095, .47, .80, -.65, 1.65, (1.05, .96, 1.10)),
)


class Aquarium:
    def __init__(self):
        self.models = [Jellyfish(loop_period=PERIOD * d.speed) for d in DRIFTERS]
        self.shader = Jellyfish()

    def kinematics(self, index, elapsed):
        """Return a path, a tangent-aligned body frame, and its velocity."""
        d, model = DRIFTERS[index], self.models[index]
        elapsed = np.asarray(elapsed)
        local_time = elapsed * d.speed + d.pulse_phase
        bottom = 3.65 + 1.30 * d.size
        top = -4.20 - 5.20 * d.size
        travel = bottom - top
        stroke_gain = .80 * d.size
        # A squeeze changes progress along the path, never an independent
        # horizontal or vertical displacement. Positive du means no backstroke.
        raw = elapsed / PERIOD + d.entry_phase - stroke_gain * model.swimming.position(local_time) / travel
        u = np.mod(raw, 1.0)
        du = 1 / PERIOD - stroke_gain * d.speed * model.swimming.speed(local_time) / travel
        a = math.tau * u + d.curve_phase
        x = d.lane + d.lean * (u - .5) + d.sway * np.sin(a)
        y = bottom - travel * u
        z = d.depth + .24 * np.sin(a + .9)
        tangent = np.stack((d.lean + math.tau * d.sway * np.cos(a),
                            np.full_like(u, -travel),
                            math.tau * .24 * np.cos(a + .9)), axis=-1)
        forward = tangent / np.linalg.norm(tangent, axis=-1, keepdims=True)
        tail = -forward
        right = np.cross(tail, np.array((0., 0., 1.)))
        right /= np.linalg.norm(right, axis=-1, keepdims=True)
        normal = np.cross(right, tail)
        spin = d.spin + .12 * np.sin(math.tau * elapsed / PERIOD + d.curve_phase)
        c, s = np.cos(spin)[..., None], np.sin(spin)[..., None]
        right, normal = right * c + normal * s, normal * c - right * s
        basis = np.stack((right, tail, normal), axis=-1)
        center = np.stack((x, y, z), axis=-1)
        velocity = tangent * du[..., None]
        return center, basis, velocity

    def projected(self, index, elapsed, width, height):
        d, model = DRIFTERS[index], self.models[index]
        local_time = elapsed * d.speed + d.pulse_phase
        xyz, emission = model.points(local_time, carry_swim=False)
        trailing = np.clip(xyz[:, 1] / 3.2, 0, 1) ** 1.4
        # A filament follows the actual past positions and orientations of
        # its bell. Its root has zero delay and stays attached during turns.
        delay = .95 * trailing
        centers, frames, _ = self.kinematics(index, elapsed - delay)
        world = centers + d.size * np.einsum('nij,nj->ni', frames, xyz)
        x, y, z = world.T
        perspective = 9 / (9 - z)
        pixels_per_unit = width / 10.0
        px = width / 2 + x * perspective * pixels_per_unit
        py = height / 2 + y * perspective * pixels_per_unit
        density_gain = (pixels_per_unit * d.size / 11.5) ** 1.6 * (8 / 2 ** 1.6)
        center_z = self.kinematics(index, elapsed)[0][2]
        fog = .61 + .35 * np.clip((center_z + 1.8) / 2.8, 0, 1)
        emission *= density_gain * fog
        emission *= np.array(d.tint)
        emission *= (.88 + .12 * np.clip(z - center_z + .5, 0, 1))[:, None]
        return px, py, emission

    def light_frame(self, width, height, elapsed):
        """Render one light buffer without depending on preceding frames."""
        dot_width, dot_height = width * 2, height * 4
        samples = [self.projected(i, elapsed, dot_width, dot_height) for i in range(len(DRIFTERS))]
        x, y, light = (np.concatenate([p[j] for p in samples]) for j in range(3))
        ix, iy = np.floor(x).astype(int), np.floor(y).astype(int)
        fx, fy = x - ix, y - iy
        pixels = np.zeros((dot_height * dot_width, 3))
        for dx, dy, weight in ((0, 0, (1-fx)*(1-fy)), (1, 0, fx*(1-fy)),
                               (0, 1, (1-fx)*fy), (1, 1, fx*fy)):
            px, py = ix + dx, iy + dy
            valid = (px >= 0) & (px < dot_width) & (py >= 0) & (py < dot_height)
            indices = py[valid] * dot_width + px[valid]
            for channel in range(3):
                pixels[:, channel] += np.bincount(indices,
                    weights=light[valid, channel] * weight[valid], minlength=dot_height * dot_width)
        return pixels.reshape(dot_height, dot_width, 3)

    def frame(self, width, height, elapsed, *, temporal=False):
        pixels = self.light_frame(width, height, elapsed)
        return self.shader.shade(pixels, width, height, elapsed, temporal=temporal)
