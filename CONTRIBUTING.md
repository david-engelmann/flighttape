# Contributing to flighttape

Thank you for contributing to `flighttape`. The crate serves as a foundational format and library across downstream agent-evaluation tools, so determinism, minimal dependencies, and exact CLI behavior are strictly enforced.

## Prerequisites

- Rust stable toolchain (`1.74` or newer) with `rustfmt` and `clippy`
- Python `3.12+` (used by `scripts/codepoint_scan.py`)

## Local Verification Gate

Run all four checks from the repository root before opening a pull request:

```sh
python3 scripts/codepoint_scan.py
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

All checks must pass with zero warnings and zero test failures.

## Engineering Rules

1. **Zero em-dashes or en-dashes**: Every Markdown and text file in the repository must contain zero `U+2014` and zero `U+2013` characters. Use plain ASCII hyphens (`-`), commas, colons, or parentheses instead. `scripts/codepoint_scan.py` enforces this rule in CI.
2. **Minimal dependency footprint**: `sha2` is the only permitted runtime dependency in `Cargo.toml`. Do not add `serde`, `clap`, `regex`, `tokio`, or other external crates.
3. **Behavioral stability**: `CONTRACT.md` and `schema/tape.schema.json` define the normative `tape_version` `"1.0"` specification, Canonical Form 1 digest rules, error messages, and `argparse`-compatible CLI semantics. Any change to observable CLI behavior, help text, or validation rules requires an explicit contract review and corresponding integration tests under `tests/`.
4. **Non-raising inputs**: Neither conversion nor validation may panic on arbitrary byte sequences, invalid UTF-8, unpaired UTF-16 surrogates, deep container nesting, or oversized numbers.
5. **Synthetic test data only**: Never commit real user transcripts, home directory paths, or credentials in `tests/fixtures/` or `examples/`. Use clearly synthetic session IDs, timestamps, and paths.
