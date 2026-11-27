<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="media/lockup-dark.svg">
    <img src="media/lockup-light.svg" alt="flighttape" width="780">
  </picture>
</p>

<p align="center">
  <a href="https://github.com/david-engelmann/flighttape/actions/workflows/ci.yml"><img src="https://github.com/david-engelmann/flighttape/actions/workflows/ci.yml/badge.svg" alt="ci"></a>
  <a href="https://github.com/david-engelmann/flighttape/actions/workflows/clean-install.yml"><img src="https://github.com/david-engelmann/flighttape/actions/workflows/clean-install.yml/badge.svg" alt="clean-install"></a>
  <a href="https://github.com/david-engelmann/flighttape/actions/workflows/docs.yml"><img src="https://github.com/david-engelmann/flighttape/actions/workflows/docs.yml/badge.svg" alt="docs"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-amber.svg" alt="MIT License"></a>
</p>

# flighttape

Every agent run, from every harness, lands as one flight tape: events, tool calls, diffs, spend, timings, and the outcome, in a bundle any tool can read back.

Flighttape defines one bundle format for a finished agent run, and the tools that work over it. The format is specified in CONTRACT.md and in the schema at schema/tape.schema.json. A harness session file becomes a tape once, and every reader after that works on the tape instead of reverse engineering each harness on its own. The converter reads Claude Code session files. Flighttape records the run, onrecord remembers across runs.

<p align="center">
  <img src="media/money-shot.gif" alt="flighttape CLI convert, validate, play, and screen showcase" width="880">
</p>

## Install

### Prebuilt Binary (Linux & macOS)

```sh
curl -fsSL https://raw.githubusercontent.com/david-engelmann/flighttape/main/install.sh | sh
```

### Cargo Binstall or Crates.io / Source

```sh
# Prebuilt binary via cargo-binstall
cargo binstall flighttape

# Build from source (Rust 1.74+)
git clone https://github.com/david-engelmann/flighttape.git
cd flighttape
cargo install --path . --locked
```

Shell completions for `bash`, `zsh`, `fish`, and `PowerShell` ship under [`completions/`](completions/).

## Quickstart

The repository ships a small synthetic Claude Code session for trying the tool. Convert it to a tape, then check the tape and print its digest.

```sh
flighttape convert tests/fixtures/claude_session_synthetic.jsonl -o example.tape.json
flighttape validate example.tape.json
valid tape, 5 events
flighttape digest example.tape.json
cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3
```

## Architecture

<p align="center">
  <img src="media/architecture.svg" alt="flighttape Canonical Form 1 architecture" width="900">
</p>

## Usage

Copy `examples/session-a.jsonl` and `examples/session-b.jsonl` into a working directory (and into a `sessions/` subdirectory) to exercise all ten subcommands:

```sh
mkdir tapes
flighttape convert session-a.jsonl -o tapes/run-a.tape.json
flighttape convert session-b.jsonl -o tapes/run-b.tape.json
flighttape convert-dir sessions -o converted
flighttape validate tapes/run-a.tape.json
flighttape verify tapes/run-a.tape.json a88422b41393c0f6be88b624ce7595c1615af3c8eb0af3ae3d15dda622080e79
flighttape play tapes/run-a.tape.json
flighttape compare tapes/run-a.tape.json tapes/run-b.tape.json
flighttape rollup tapes
flighttape slice tapes/run-a.tape.json --from 0 --to 1 -o opening.tape.json
flighttape screen tapes/run-a.tape.json
```

See the full walkthrough of every subcommand, JSON output payload, and exit code in [docs/GUIDE.md](docs/GUIDE.md).

## Performance

Measured on Apple M3 Max (`--release`, median of 5 runs; full methodology in [docs/PERFORMANCE.md](docs/PERFORMANCE.md)):

| Events | Tape Size | `convert` | `validate` | `screen` | Throughput (`validate`) |
| ---: | ---: | ---: | ---: | ---: | ---: |
| `200` | `44.0 KiB` | `28.89 ms` | `18.65 ms` | `27.96 ms` | `10,724 ev/s` |
| `2,000` | `438.7 KiB` | `492.83 ms` | `82.06 ms` | `143.91 ms` | `24,372 ev/s` |
| `10,000` | `2204.3 KiB` | `1307.24 ms` | `303.5 ms` | `250.23 ms` | `32,949 ev/s` |

## Where `flighttape` Fits in the Retrospective Line

<p align="center">
  <img src="media/retrospective-arc.svg" alt="Retrospective Line ten-repository architecture" width="900">
</p>

## Documentation

- [Documentation Index](docs/README.md) & [Searchable Handbook](https://david-engelmann.github.io/flighttape/)
- [Operator & Integrator Guide](docs/GUIDE.md)
- [Technical Architecture](docs/ARCHITECTURE.md)
- [Wire Formats & Exit Codes](docs/FORMATS.md)
- [Ecosystem Comparison](docs/COMPARISON.md)
- [Performance Benchmarks](docs/PERFORMANCE.md)
- [FAQ](docs/FAQ.md)

## Status

The tape format and all ten commands are built and run as a Rust binary, with the Python of the first ten rounds standing as the behavior reference the port is pinned against. The converter reads Claude Code session files. Converters for other harnesses are not built yet, and the format is the part that is meant to outlast any one harness.

Closed perfection rounds, 49 of 100. A round counts only when a full cycle of work, audit, review, red team, and approval has closed.

## License

MIT. See [LICENSE](LICENSE).
