# Flighttape Documentation

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="media/lockup-dark.svg">
    <img src="media/lockup-light.svg" alt="flighttape" width="760">
  </picture>
</p>

Welcome to the documentation suite for `flighttape`, the canonical `1.0` run bundle format, Canonical Form 1 digest engine, and ten-subcommand Rust CLI and library for finished AI agent runs.

## Documentation Index

| Document | Audience | Contents |
| :--- | :--- | :--- |
| [Operator & Integrator Guide](GUIDE.md) | CLI users, platform teams, harness authors | Installation (`install.sh`, `cargo binstall`, source), all 10 subcommands, redaction vs screening, Rust library API, adapter spec |
| [Technical Architecture](ARCHITECTURE.md) | Systems engineers & contributors | Byte ingestion, path redactor, Canonical Form 1 serializer, 512 MiB parser stack, atomic write guard |
| [Wire Formats & Schemas](FORMATS.md) | Downstream tool authors | `tape_version` `"1.0"` field contract, `compare`/`rollup`/`screen` JSON shapes, exit codes `0`/`1`/`2`, `schema/tape.schema.json` |
| [Ecosystem Comparison](COMPARISON.md) | Evaluators & architects | How `flighttape` differs from OpenTelemetry spans, LangSmith/Langfuse SaaS traces, raw harness JSONL, and `onrecord` |
| [Performance & Scale](PERFORMANCE.md) | Infrastructure teams | Reproducible synthetic corpus benchmarks (`200` to `10,000` events) on Apple Silicon (`32,949` events/sec validation) |
| [Frequently Asked Questions](FAQ.md) | All users | Redaction limits, multi-harness support, Canonical Form 1 rules, exit code `1` vs `2`, CI gating |
| [Storefront README Draft](STOREFRONT_README_DRAFT.md) | Maintainers & Muse seat | Drop-in visual storefront upgrade preserving all contracted `README.md` sections and round pins |

## Machine-Readable Contracts

- JSON Schema (Draft 2020-12): [`../schema/tape.schema.json`](../schema/tape.schema.json)
- Behavioral Contract: [`../CONTRACT.md`](../CONTRACT.md)
- Shell Completions: [`../completions/`](../completions/) (`bash`, `zsh`, `fish`, `PowerShell`)
