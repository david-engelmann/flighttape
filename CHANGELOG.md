# Changelog

All notable changes to `flighttape` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-11-27

### Added

- Canonical flight tape bundle specification (`tape_version` `"1.0"`) and JSON Schema (`schema/tape.schema.json`) for recording finished AI agent runs.
- Canonical Form 1 (`identity` module) providing language-portable byte serialization and SHA-256 tape digests over UTF-16 key ordering, exact arbitrary-length integers up to 4,300 digits, and ECMAScript shortest round-trip floating-point numbers.
- Ten CLI subcommands (`convert`, `convert-dir`, `validate`, `play`, `digest`, `verify`, `compare`, `rollup`, `slice`, and `screen`) with `argparse`-compatible option parsing and deterministic exit codes.
- Resilient Claude Code JSONL session converter (`claude-code-jsonl`) with non-raising line recovery, `run.skipped_lines` accounting, automatic POSIX absolute path redaction, and rule-derived run outcomes (`completed`, `failed`, `aborted`, `unknown`).
- Iterative JSON parser, sanitizer, validator, and path screener hardened for deep container nesting (up to 1,000 levels) and atomic report output (`screen -o`).
- Rust library crate (`flighttape`) with `sha2` as its sole external runtime dependency.
