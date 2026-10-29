# Offscreen rejection and fixed-cell painting trial

## Decision

Neither candidate was promoted to the default branch or installed commands.
The aquarium's export latency did not show a clear improvement. Removing
work is not sufficient evidence that an optimization helps the complete
pipeline. The existing v0.4.1 release remains the production version.

The trial source is retained on `perf/offscreen-glyph-trial` for review.

## Implemented candidates

1. Conservative world-space bounds include the whole anatomy and the delayed
   path interval [t-.95,t]. Entirely offscreen animals skip point generation
   and projection; temporal shading still runs normally. Ambiguous recycle
   boundaries, near-camera bounds, and unusual numeric inputs render normally.
   An explicit one-dot splat halo is included in the frustum test.
2. The embedded glyphs use sparse lists of nonzero mask pixels. Opaque pixels
   copy their color, and translucent pixels reuse the exact integer blend
   formula. The optimized path requires a fully contained fixed grid; custom
   fonts, overlap, and clipping retain the general painter.

The geometric enclosure uses local y in [-1.1,3.5], radial distance <=1.8,
positive tail-y orientation, bounded tail-x/z components, and the swimming
lookup table's actual extrema. The global interval is widened for rounding,
and wraparound is handled conservatively by skipping the rejection test.
These deliberately broad bounds are a first prototype, not a minimum box.

## Measurements

Three complete exports per variant and preset, four render workers on the
same WSL machine. Order rotated each round; compilation is excluded. The
baseline executable is the existing v0.4.1. This run's absolute times differ
from earlier sessions; compare variants within this trial only.

| Scene / variant | Median wall | Median aggregate CPU |
| --- | ---: | ---: |
| aquarium / baseline | 18.083 s | 64.259 s |
| aquarium / culling | 17.957 s | 61.974 s |
| aquarium / both | 18.549 s | 62.201 s |
| single / baseline | 6.778 s | 12.806 s |
| single / culling | 6.834 s | 13.009 s |
| single / both | 6.598 s | 12.676 s |

For the aquarium, rejection alone changed median CPU time by -3.6%,
but its wall-time change was small. Combining both prototypes changed median
wall time by +2.6%. The single-scene results also did not establish
an improvement large enough to justify enabling the new path by default.
No significance test or general performance guarantee is claimed.

All 18 comparison GIFs were byte-identical within their preset:

```text
aquarium a7cf28b4bbe400feaeb341acb0aa9a033633af7af1e1006cc2e13dce93d62cec
single   aa36ee7c71f53fb3c1ccaa306887d7b28287af23b5c445d3f7cd485b7621d0ef
```

[Raw export measurements](benchmarks/offscreen-glyph-trial.json).

A separate 48-frame serial CPU-stage sample covers three sections of each
loop, excluding FFmpeg and pipeline waiting. It suggested light computation
was the primary aquarium cost, while painting accounted for a larger share
of the single scene. This sample is diagnostic, not a full-export speedup:
[aquarium](benchmarks/offscreen-glyph-profile-aquarium.json),
[single](benchmarks/offscreen-glyph-profile-single.json).

## Verification

Local Rust tests passed: seven unit tests and fifteen parity/integration
checks. Added cases cover anatomical enclosure, rejection against actual
projected points at three aspect ratios, uncertain inputs, every embedded
Braille glyph, and clipped fallback behavior. Existing Python-generated color,
geometry, RGB hashes, custom-font overlap, ANSI output, and resize fixtures
remain unchanged. Cross-platform release builds were not requested for these
unadopted prototypes.

## Reproduction on this branch

Build the baseline v0.4.1 and the candidate snapshots separately:

- Baseline: v0.4.1 tag.
- Rejection only: commit b4e128a.
- Both candidates: commit 9cbe940.

```sh
cargo build --locked --release --bins
python3 tools/benchmark-native.py --baseline-ref v0.4.1 \
  --binary baseline=/path/to/v0.4.1/kurage-export \
  --binary culling=/path/to/b4e128a/kurage-export \
  --binary both=/path/to/9cbe940/kurage-export \
  --output output/offscreen-glyph.json
cargo run --locked --release --example profile
cargo run --locked --release --example profile -- --single
```

These changes are experimental. Further work would need a new bounded trial
and the same complete-GIF equality gate before promotion.
