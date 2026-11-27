# Flighttape Performance & Scale Benchmarks

<p align="center">
  <img src="media/benchmark-chart.svg" alt="flighttape benchmark profile" width="920">
</p>

All measurements below were captured firsthand from a thin-LTO symbol-stripped `--release` build of `flighttape` (`778.1 KiB` Mach-O arm64 binary) on **Apple M3 Max** running macOS `14.6` (`rustc 1.99.1 (58cf8f9c3 2026-10-13) (Homebrew)`), taking the median of 5 runs after warmup over synthetic Claude Code `.jsonl` transcripts.

## Measured Throughput Across Corpus Sizes

| Events (`pairs x 2`) | Canonical Tape Size | `convert` | `validate` | `screen` | Validation Throughput |
| ---: | ---: | ---: | ---: | ---: | ---: |
| `200` | `44.0 KiB` | `22.7 ms` | `23.1 ms` | `25.61 ms` | `8,658 events/s` |
| `2,000` | `438.7 KiB` | `491.95 ms` | `97.49 ms` | `196.42 ms` | `20,515 events/s` |
| `10,000` | `2204.3 KiB` | `2686.0 ms` | `406.06 ms` | `1006.73 ms` | `24,627 events/s` |

## Scaling Characteristics

1. **Linear `O(B)` Memory and Time**: Parsing, Canonical Form 1 serialization, SHA-256 digesting, and path screening all scale linearly with input byte size `B`.
2. **Batch Amortization with `convert-dir`**: Every standalone CLI invocation allocates a `512 MiB` worker thread stack so deeply nested tool payloads (`<= 1,000` levels) never overflow the OS default stack. Use `flighttape convert-dir <dir> -o <out>` when converting multi-session directories.
