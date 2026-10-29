# Rust migration (v0.4.0)

## Boundary and completion

Both user-facing commands, including geometry, swimming, projection, Braille
and ASCII shading, temporal filtering, live terminal output, glyph painting,
worker orchestration, and PNG writing, run in Rust. FFmpeg remains the external
GIF encoder. There is no Python subprocess, extension module, or fallback.
The Python package and its entry points were removed in the same cutover.

The public CLI flags remain available, including `--single`, `--workers`,
`--font`, `--ascii`, `--static`, `--color`, `--no-color`, `--time`, and `--fps`.
Static output, redirected output, `NO_COLOR`, resize, and cursor restoration
are part of the terminal contract. This viewer does not enter an alternate
screen or change mouse bindings.

## Oracle and test migration

The pinned oracle is Python release v0.3.0, commit `c5d795a`, using Python 3.12,
NumPy 2.5.4, Pillow 13.0.0, and DejaVu Sans. `tools/oracle.py` reads that commit
through `git archive`; it never imports the current Rust implementation.
The generated fixture records the exact Python patch version.

- 20 frame sequences / 180 frames cover both character modes, three scene
  configurations, four viewport sizes, negative time, consecutive frames,
  gaps, repeated loop boundaries, and temporal resets.
- 54 poses and sampled anatomical points check floating-point geometry.
- Six full-resolution frames check pre-quantization RGB SHA-256 values.
- Nine custom-font cases cover every Braille glyph, offsets, clipping, and
  overlapping glyphs at three font sizes.
- All eight original test names have corresponding Rust tests. The inventory
  gate checks for omitted cases; there are no pending or ignored migration tests.
- Native checks cover CLI validation without Python in PATH, parallel ownership,
  and encoder failure cleanup. Shared-memory-specific tests were replaced by
  owned-buffer tests because shared memory was removed.

To regenerate the historical fixtures (optional development tooling):

```sh
uv venv .venv
uv pip install --python .venv/bin/python numpy==2.5.4 Pillow==13.0.0
.venv/bin/python tools/oracle.py
git diff --exit-code -- tests/fixtures assets/render
```

Use Python 3.12 for this command. The runtime, normal build, and Rust test suite
do not require that environment. Oracle regeneration was checked for zero diff.

## Full-export replay and performance

On one WSL/Linux machine with 16 available logical CPUs, both implementations
exported each complete scene three times using four rendering workers. The
implementation order alternated between runs. Wall time includes startup,
rendering, first-frame PNG, and FFmpeg encoding; compilation is excluded. CPU
time is summed user + system time for waited child processes and their workers.

| Full scene | Python median wall | Rust median wall | Python median CPU | Rust median CPU |
| --- | ---: | ---: | ---: | ---: |
| Giant aquarium, 1280×720, 1152 frames | 20.06 s | 11.72 s | 100.38 s | 36.31 s |
| Single jellyfish, 1280×720, 576 frames | 5.57 s | 3.56 s | 15.72 s | 7.09 s |

The aquarium took 41.6% less wall time and 63.8% less CPU time. The single
scene took 36.1% less wall time. These are observations on this machine, not
guarantees for other CPUs or operating systems. Raw measurements are in
[benchmarks/rust-v0.4.0.json](benchmarks/rust-v0.4.0.json).

The toolchain was rustc 1.97.1 (release profile, thin LTO), Python 3.12,
NumPy 2.5.4, Pillow 13.0.0, and FFmpeg 6.1.1-3ubuntu5. Each scene's GIF was
byte-identical across all six Python/Rust runs:

```text
aquarium a7cf28b4bbe400feaeb341acb0aa9a033633af7af1e1006cc2e13dce93d62cec
single   aa36ee7c71f53fb3c1ccaa306887d7b28287af23b5c445d3f7cd485b7621d0ef
```

Reproduce on Unix with the pinned Python dependencies installed:

```sh
cargo build --locked --release --bins
.venv/bin/python tools/benchmark.py --python .venv/bin/python
```

The benchmark restores the old source in a temporary directory, runs both
presets, compares SHA-256 values, and removes temporary renderings. Existing
background settings and the active GIF do not need replacement.

## Compatibility decisions

| Behavior | Source | Rust decision and check |
| --- | --- | --- |
| Negative animation time | NumPy positive remainder | `rem_euclid`; negative-time fixture sequences |
| Rounding | Python/NumPy ties to even | `round_ties_even`; exact colors and ANSI fixtures |
| Geometry and splatting | float64, ordered coverage passes | float64; numeric tolerance 1e-12 internally, exact visible fixture output |
| Temporal state | Ordered shading after parallel rendering | Same order; bounded owned buffers and thread pool |
| Default glyphs | Pillow/DejaVu Sans masks | Embedded generated coverage; exact RGB frame hashes |
| Default palette | Pillow samples the fixed scene before encoding | Same generated palette embedded as a versioned scene asset; full GIF replay exact |
| Custom font rasterization | Pillow/FreeType | Bundled FreeType; nine full-image hash comparisons |
| Custom font palette | Recomputed from that font | Uses the embedded scene palette; custom-font GIF byte identity is not claimed |
| PNG file | Pillow compression/metadata | Rust PNG encoder; decoded RGB is checked, PNG file bytes may differ |
| Errors | argparse / Python exceptions | clap usage errors and concise runtime errors; nonzero exit status |
| Worker failure | Process/shared-memory cleanup | Owned buffers and child/temp guards; failure integration test |

The positive remainder, ties-to-even rounding, and coverage-pass order are
permanent rendering semantics, not temporary shims. Embedded palettes belong to
the two fixed presets; changing scene colors should update their palette assets
and replay fixtures together. There is no transitional Python compatibility layer
to delete. FreeType 0.36 bindings were selected because their bundled build
works without system font libraries; 0.38's bundled sys crate required zlib
headers that were absent in the build environment.

References: [NumPy interpolation](https://numpy.org/doc/stable/reference/generated/numpy.interp.html),
[NumPy rounding](https://numpy.org/doc/stable/reference/generated/numpy.rint.html),
[Rust float methods](https://doc.rust-lang.org/std/primitive.f64.html).

## Cutover and rollback

The complete default exports are the replay workload for this offline CLI;
server traffic shadowing and percentage canaries do not apply. Native binaries
replace the local Python launcher together after tests and full-export replay.
GitHub Actions builds and tests Linux x86-64, Windows x86-64, macOS Apple Silicon,
and macOS Intel. GUI background playback is separate from generator validation.

The immutable v0.3.0 tag and release remain available. To run the old generator,
extract that tag into a separate directory and install its pinned Python
dependencies there. The benchmark exercises this source-restoration path on
both complete scenes. Keep the old release while evaluating the native one;
no intermediate mixed-language release is published.
