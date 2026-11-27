# Flighttape Technical Architecture

<p align="center">
  <img src="media/architecture.svg" alt="flighttape technical architecture" width="920">
</p>

`flighttape` is built around a single architectural premise: **every agent harness logs differently, so downstream analysis tools must never parse raw harness logs directly**. Instead, raw session transcripts cross a strict normalization and redaction boundary once, producing an immutable, content-addressed `1.0` JSON bundle (`tape_digest`) that every downstream tool can verify in milliseconds.

## 1. Crate & Module Topology

`flighttape` compiles as both a library crate (`flighttape`) and a standalone CLI binary (`src/bin/flighttape.rs`) with only one runtime dependency (`sha2` for FIPS 180-4 SHA-256). Zero `serde`, `clap`, `tokio`, or regex crates sit in the dependency tree.

| Layer | Responsibility | Key Invariants |
| :--- | :--- | :--- |
| **Byte & JSON Reader** | Custom recursive-descent JSON parser over raw byte slices | Enforces explicit nesting depth cap (`<= 1,000`), rejects lone UTF-16 surrogates, preserves exact integer and float lexical forms |
| **Claude Code Converter** | Stateful session stream normalizer (`convert`, `convert-dir`) | Filters foreign `sessionId` records, tallies malformed/skipped lines in `run.skipped_lines`, pairs `tool_call` and `tool_result` IDs, enforces monotonic `seq` (`0..N-1`) and non-decreasing timestamps |
| **Path Redactor & Screener** | Automatic conversion-time redaction + pre-share audit (`screen`) | Replaces multi-segment POSIX (`/Users/alice/repo`) and Windows (`C:\Users\alice\repo`) paths with `<REDACTED>` while counting `redacted_values`; `screen` checks all string leaves and exits `1` if any path survives |
| **Canonical Form 1 Engine** | Deterministic JSON serialization & SHA-256 hashing (`digest`, `verify`) | Sorts object keys lexicographically, emits minimal UTF-8 JSON without insignificant whitespace, computes 64-char lowercase hex SHA-256 over the tape minus `tape_digest` |
| **Timeline, Diff & Rollup** | Read-only analytical projections (`play`, `compare`, `rollup`, `slice`) | Pure functions over validated `1.0` bundles; `slice` re-numbers `seq` from `0` and recomputes a fresh `tape_digest` |
| **Filesystem Guard** | Input-preservation & atomic output writer | Resolves canonical paths/inodes so `-o` never overwrites the input file; writes to a sibling tempfile and atomically renames into place |

## 2. Thread Stack & Adversarial Nesting Defense

Real-world tool outputs inside agent transcripts occasionally embed deeply nested AST dumps or JSON payloads. `flighttape` runs its CLI entrypoint on an explicit `512 MiB` worker thread stack while capping JSON value depth at `1,000` levels, guaranteeing deterministic refusal (`defect, ...`) rather than process-aborting stack overflows on adversarial inputs.

## 3. Position in the Retrospective Line

<p align="center">
  <img src="media/retrospective-arc.svg" alt="flighttape in the Retrospective Line" width="920">
</p>

- **Upstream**: Agent harnesses (Claude Code `.jsonl` natively today; Codex, Cursor, Gemini CLI, and OpenCode via the adapter contract in [GUIDE.md](GUIDE.md)).
- **Sibling library consumers**: `secondtake` (uses `flighttape`'s JSON reader and Canonical Form 1 digest for first-prompt task keys) and `onrecord` (reuses `flighttape::timeparse` and Canonical Form 1 digesting for position-bound claim IDs).
- **Downstream tape readers**: `unheard`, `retroforge`, and `secondtake`.
