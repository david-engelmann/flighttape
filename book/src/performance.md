# Flighttape Performance & Scale Benchmarks

All measurements below were captured firsthand from a `--release` build of `flighttape` (`972.4 KiB` Mach-O arm64 binary) on **Apple M3 Max** running macOS `14.6` (`rustc 1.99.1 (58cf8f9c3 2026-10-13) (Homebrew)`), taking the median of 5 runs after warmup over synthetic Claude Code `.jsonl` transcripts.

## Measured Throughput Across Corpus Sizes

| Events (`pairs x 2`) | Canonical Tape Size | `convert` (JSONL -> `1.0` + redact + digest) | `validate` (full schema + seq + digest check) | `screen` (all string leaves) | Validation Throughput |
| ---: | ---: | ---: | ---: | ---: | ---: |
| `200` | `44.0 KiB` | `28.89 ms` | `18.65 ms` | `27.96 ms` | `10,724 events/s` |
| `2,000` | `438.7 KiB` | `492.83 ms` | `82.06 ms` | `143.91 ms` | `24,372 events/s` |
| `10,000` | `2204.3 KiB` | `1307.24 ms` | `303.5 ms` | `250.23 ms` | `32,949 events/s` |

## Scaling Characteristics

1. **Linear `O(B)` Memory and Time**: Parsing, Canonical Form 1 serialization, SHA-256 digesting, and path screening all scale linearly with input byte size `B`. Validating a 10,000-event (`2.2 MiB`) session bundle completes in `303.5 ms`.
2. **Batch Amortization with `convert-dir`**: Every standalone CLI invocation spawns a dedicated `512 MiB` stack worker thread so deeply nested tool payloads (`<= 1,000` levels) never overflow the OS default stack. When converting dozens or hundreds of raw `.jsonl` sessions, prefer `flighttape convert-dir <dir> -o <out>` to amortize process startup across the entire batch.
3. **Zero Network / Zero Warmup**: Because `flighttape` makes zero network calls and loads no dynamic runtimes, cold-start CLI execution on typical 50-to-200-event sessions finishes in `18-30 ms`.
