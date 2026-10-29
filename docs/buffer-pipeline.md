# Reusable buffers and overlapping export stages (v0.4.1)

This change implements two bounded optimizations: reusing working storage and
overlapping light-buffer computation with ordered shading, image painting, and
FFmpeg input. Resolution, frame count, timing, palette, geometry, and arithmetic
order are preserved. FFmpeg and its encoder options are unchanged.

## Implementation

`LightWorkspace` retains anatomical points, projected points, the coverage-pass
buffer, and the filament pulse cache. The light output buffer is reused too.
`Shader::shade_into` retains its brightness, strength, active-dot, glyph, and
color buffers. It updates temporal history while blending instead of copying
the entire blended light buffer afterward. `Painter::draw_into` fills an existing
RGB image. The original allocating APIs remain available for callers.

The renderer cycles two light-buffer batches through bounded ready/recycle
queues. While one batch is shaded, painted, and written, the Rayon pool prepares
the next. A scoped coordinator owns the scratch buffers, and the consumer keeps
temporal shading strictly in frame order. Both batches together hold at most
32 MiB of raw light data for the fixed export presets. Scratch and RGB storage
are additional, and the full animation is never retained in memory.

If the consumer fails or export is cancelled, queue endpoints are dropped before
joining the producer. This releases blocked sends/receives and allows the FFmpeg
child and temporary directory guards to finish normally.

## Measurements

Three complete exports per implementation and preset, on the same WSL machine
with 16 available logical CPUs, four render workers, Rust release builds, and
FFmpeg 6.1.1. Implementation order rotated each round. Compilation is excluded;
startup, first-frame PNG, rendering, and GIF encoding are included.

| Scene / implementation | Median wall | Median CPU |
| --- | ---: | ---: |
| Giant aquarium: v0.4.0 | 11.547 s | 36.705 s |
| Giant aquarium: buffers only | 11.426 s | 35.936 s |
| Giant aquarium: buffers + pipeline | 10.675 s | 37.135 s |
| Single: v0.4.0 | 3.642 s | 7.123 s |
| Single: buffers only | 3.731 s | 7.146 s |
| Single: buffers + pipeline | 3.530 s | 7.007 s |

The combined aquarium export used 7.6% less wall time and 1.2% more aggregate
CPU time. The single export used 3.1% less wall time. Buffer reuse by itself did
not show a clear speed improvement. These observations do not establish a
general speedup for other machines or a reduction in CPU work.

All 18 GIFs were byte-identical within their respective scene:

```text
aquarium a7cf28b4bbe400feaeb341acb0aa9a033633af7af1e1006cc2e13dce93d62cec
single   aa36ee7c71f53fb3c1ccaa306887d7b28287af23b5c445d3f7cd485b7621d0ef
```

In a separate one-run memory check, generator-only peak RSS was 66,164 KiB for
v0.4.0 and 68,184 KiB for both buffer-only and pipelined versions: about 2 MiB
higher, not lower. Retaining reusable storage reduces repeated allocations but
does not imply a lower memory high-water mark. The sample reads Linux `VmHWM`
every 20 ms and excludes the FFmpeg process. It is not aggregate process-tree
memory or a multi-run memory benchmark.

Raw data: [timings](benchmarks/buffer-pipeline-v0.4.1.json),
[memory check](benchmarks/buffer-pipeline-memory-v0.4.1.json).
The reusable buffers are checked for stable backing storage after warmup;
speed and memory claims above remain limited to the measurements.

## Rejected follow-up

A prototype generated anatomical points directly into projection, removing the
point buffer. It preserved complete GIF output but increased aquarium CPU time:
37.80 s median for its contemporaneous baseline, 39.06 s for its buffer-only
variant, and 40.76 s with its pipeline. It did not establish a clear additional
wall-time benefit over the accepted pipeline in the earlier trial. This prototype
was discarded rather than treating fewer arrays as proof of better performance.
The reason for the extra CPU cost was not isolated. Its
[measurements](benchmarks/point-streaming-rejected-v0.4.1.json) are retained.

## Reproduction

Build v0.4.0 and the current version in separate directories, then run:

```sh
python3 tools/benchmark-native.py \
  --binary baseline=/path/to/v0.4.0/kurage-export \
  --binary pipeline=target/release/kurage-export
```

The buffer-only checkpoint is commit `c5bff86`. The benchmark records executable
SHA-256 values, rotates the order, and compares complete GIF hashes. Use
`--runs 1 --rss --output output/memory.json` for the separate Linux memory check.
The committed timing executables were captured before the version-only bump.

## Verification

The existing Python-generated geometry, color, text, image-hash, and resize
fixtures still pass. Added checks exercise reusable backing storage, mutated
buffer recycling, partial batches, ordered frames with 1/2/4 render threads,
consumer failure, cancellation, and producer shutdown without deadlock. The
custom-font clipping/overlap checks reuse the RGB buffer across cases.

Channels follow the documented [Rust bounded-channel semantics](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html).
