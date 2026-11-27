<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="media/lockup-dark.svg">
    <img src="media/lockup-light.svg" alt="flighttape" width="760">
  </picture>
</p>

<p align="center">
  <img src="media/money-shot.gif" alt="flighttape verified CLI workflow" width="880">
</p>

<p align="center">
  <img src="media/architecture.svg" alt="flighttape technical architecture" width="900">
</p>

# Flighttape Operator and Integrator Guide

Flighttape defines one canonical JSON bundle (`tape_version` `"1.0"`) for a finished AI agent run, together with a ten-subcommand CLI and Rust library (`flighttape`) for converting, validating, digesting, playing back, comparing, rolling up, slicing, and screening recorded runs.

Within the broader agent-telemetry stack, `flighttape` records a single finished run as an immutable, content-addressed artifact, whereas `onrecord` tracks claims across runs and downstream evaluators (`secondtake`, `retroforge`, `catchledger`, `forkdrift`) consume validated `1.0` tapes directly.

## Contents

1. [Installation](#1-installation)
2. [CLI Reference (All 10 Subcommands)](#2-cli-reference-all-10-subcommands)
3. [Path Redaction and Screening Boundaries](#3-path-redaction-and-screening-boundaries)
4. [Canonical Form 1 and Tape Digests](#4-canonical-form-1-and-tape-digests)
5. [Rust Library Embedding](#5-rust-library-embedding)
6. [Third-Party Harness Adapter Specification](#6-third-party-harness-adapter-specification)

---

## 1. Installation

### Prebuilt Release Binary (`install.sh`)

On Linux (`x86_64`, `aarch64`) and macOS (`x86_64`, `aarch64`), `install.sh` downloads the matching release archive from GitHub Releases, verifies its SHA-256 digest against `SHA256SUMS` (`sha256sum` or `shasum -a 256`), and installs `flighttape` to `$HOME/.local/bin`:

```sh
curl -fsSL https://raw.githubusercontent.com/david-engelmann/flighttape/main/install.sh | sh
```

Pin a specific release tag or choose a custom install directory via environment variables:

```sh
curl -fsSL https://raw.githubusercontent.com/david-engelmann/flighttape/main/install.sh | \
  FLIGHTTAPE_VERSION=v1.0.0 FLIGHTTAPE_INSTALL_DIR=/usr/local/bin sh
```

On Windows (`x86_64-pc-windows-msvc`), download `flighttape-x86_64-pc-windows-msvc.zip` and `SHA256SUMS` from [GitHub Releases](https://github.com/david-engelmann/flighttape/releases), verify the archive with `Get-FileHash -Algorithm SHA256`, and place `flighttape.exe` on `PATH`.

### From Crates.io or Source (`cargo install`)

With a Rust `1.78+` toolchain installed:

```sh
# Install from crates.io once published
cargo install flighttape --locked

# Or install from a local git checkout
git clone https://github.com/david-engelmann/flighttape.git
cd flighttape
cargo install --path . --locked
```

---

## 2. CLI Reference (All 10 Subcommands)

### Exit Code Conventions

Every `flighttape` invocation uses three deterministic exit codes:

- `0`: Command succeeded (`validate` passed, `verify` matched, `rollup` excluded zero files, `screen` found zero paths).
- `1`: Operational refusal, validation defect, digest mismatch, excluded rollup entries, or `screen` path hits (`clean: false`). Errors print to `stderr` prefixed with `error, ` or `defect, `, never raw stack traces.
- `2`: Command-line usage error (missing required arguments, unknown options, or invalid subcommand choice). Option parsing matches Python `argparse` semantics, including `--opt=value`, `-ovalue`, `--` positional separators, and unambiguous long-option prefixes (such as `--f` for `--from`).

### 2.1 `convert`

Convert one Claude Code JSONL session file into a validated `1.0` tape.

```sh
flighttape convert [-o OUTPUT] source
```

- Reads `source` as raw bytes; lines that are invalid UTF-8, invalid JSON, non-objects, nested past depth 1,000, from a foreign `sessionId`, or non-message events are counted in `run.skipped_lines` rather than aborting conversion.
- Redacts multi-segment POSIX absolute paths in `message.text`, `tool_result.content`, and `tool_call.input`, incrementing `run.redacted_values`.
- Validates the resulting tape before emitting it (for example, an empty session file produces zero events, fails the `events` non-empty check, prints `defect, events must hold at least one event`, and exits `1`).
- Writes 2-space indented JSON to `stdout` when `-o` / `--output` is omitted, or to `OUTPUT` when specified.

### 2.2 `convert-dir`

Convert every `.jsonl` file in `source_dir` (sorted lexicographically by filename) into `<stem>.tape.json` files inside `OUTPUT_DIR`.

```sh
flighttape convert-dir -o OUTPUT_DIR source_dir
```

- `-o` / `--output-dir` is required and created automatically if absent.
- Non-regular `.jsonl` entries (such as directories named `*.jsonl`), unreadable files, and sessions that fail validation are recorded as failures without stopping the batch.
- Writes `OUTPUT_DIR/summary.json` containing `converted`, `failed`, `events`, and per-file `files` entries, and prints `converted <N>, failed <M>, events <E>` to `stdout`.
- Exits `0` if `failed == 0`, or `1` if any `.jsonl` entry failed.
- **Rollup interaction note**: Because `convert-dir` writes `summary.json` alongside `<stem>.tape.json`, running `flighttape rollup OUTPUT_DIR` directly over that directory inspects `summary.json` (which ends in `.json`), classifies it as `invalid` in `excluded`, and exits `1`. Move or write tapes into a dedicated tape directory if you want a clean `0` rollup exit code.

### 2.3 `validate`

Check a JSON file against all structural, schema, pairing, timestamp, and numeric finiteness rules of `tape_version` `"1.0"`.

```sh
flighttape validate tape
```

- Collects **all** defects in a single pass rather than stopping at the first error.
- Prints `valid tape, <N> events` to `stdout` and exits `0` when valid; prints `defect, <reason>` lines (or `error, cannot read the tape file, <path>`) to `stderr` and exits `1` otherwise.

### 2.4 `play`

Render a validated tape as a human-readable chronological timeline.

```sh
flighttape play tape
```

- Validates the tape first; invalid tapes exit `1` with zero playback lines written to `stdout`.
- Prints a 3-line run header (`tape <session_id> from <harness>`, `outcome ...` with token/message/tool counts, and `started ...`), followed by `[+0.00s] #0 ...` event entries showing relative offsets from the first parseable timestamp.

### 2.5 `digest`

Print the 64-character lowercase hex SHA-256 digest of a file under Canonical Form 1.

```sh
flighttape digest tape
```

- Intentionally skips `validate_tape`: any JSON value representable in Canonical Form 1 (including arbitrary JSON objects or `null`) can be digested.
- Fails with exit `1` if the file cannot be read, is not valid UTF-8/JSON, nests deeper than 1,000 containers, holds an integer over 4,300 digits, contains `NaN`/`Infinity`, or holds an unpaired UTF-16 surrogate (`\ud800`..`\udfff`).

### 2.6 `verify`

Verify a JSON file against an expected SHA-256 hex digest.

```sh
flighttape verify tape expected
```

- Normalizes `expected` by trimming surrounding Unicode whitespace and lowercasing ASCII hex digits before comparing against `digest(tape)`.
- On match, prints `digest matches, <digest>` to `stdout` and exits `0`.
- On mismatch, prints `defect, digest mismatch, the tape digests to <digest>` to `stderr` and exits `1`.

### 2.7 `compare`

Compare two valid, digestible tapes (`first` as side `a`, `second` as side `b`) and emit a structured JSON diff report to `stdout`.

```sh
flighttape compare first second
```

- Validates and digests both inputs first (reporting errors on both sides if both are invalid) and exits `1` on any load, validation, or digest refusal.
- Emits a JSON object with keys:
  - `digests`: `{"a": "<sha256>", "b": "<sha256>"}`
  - `outcome`: `{"a": {...}, "b": {...}, "changed": <bool>}`
  - `duration_seconds`: `a`, `b`, and `delta` (`b - a`, omitted when either duration is `null` or overflows `f64`)
  - `measures`: union of `run.totals` keys across `a` and `b`, with exact integer `delta` where both sides are present
  - `redacted_values` and `skipped_lines`: side values and deltas
  - `events`: `count`, `by_kind`, and event index lists `changed` (shared indices whose canonical event payload differs, ignoring `seq` when comparing), `added` (indices only in `b`), and `removed` (indices only in `a`).
- Always exits `0` when both input tapes are valid and digestible, even when differences exist.

### 2.8 `rollup`

Aggregate every `.json` file in `source_dir` into a deterministic directory summary JSON report on `stdout`.

```sh
flighttape rollup source_dir
```

- Scans top-level entries ending in `.json` (case-sensitive). Regardless of filesystem directory iteration order, output arrays (`tapes` and `excluded`) are sorted deterministically by filename.
- Valid, digestible tapes appear in `tapes` with `file`, `digest`, `outcome`, `duration_seconds`, `events`, and `totals`.
- Non-tape or broken `.json` entries appear in `excluded` with `reason` set to `"not a file"`, `"unreadable"`, `"invalid"` (with `defects` array), or `"undigestible"` (with `error`). If summing a `totals` key across tapes exceeds the 4,300-digit canonical integer bound, that key is emitted as `null` rather than failing the report.
- Exits `0` when `excluded` is empty, or `1` when one or more `.json` entries were excluded.

### 2.9 `slice`

Cut a contiguous sequence window and/or event-kind subset out of a valid tape, producing a new valid tape.

```sh
flighttape slice [--from FROM_SEQ] [--to TO_SEQ] [--kind KINDS] [-o OUTPUT] tape
```

- `--from` and `--to` are inclusive 0-based integer `seq` bounds (`--from` must be `>= 0`, `--to` must be `>= 0`, and `--from <= --to`). Python-style integer literals with `_` digit separators are accepted.
- **`--kind` is repeatable**: pass `--kind` multiple times to retain several event kinds in one slice:
  ```sh
  flighttape slice run.tape.json --from 1 --to 10 --kind tool_call --kind tool_result -o tools.tape.json
  ```
  Each `--kind` value must be one of the eight valid event kinds (`session_start`, `message`, `tool_call`, `tool_result`, `diff`, `spend`, `note`, `session_end`). Note that comma-separated values like `--kind tool_call,tool_result` are treated as a single unknown kind name and refused; always repeat `--kind`.
- Slicing renumbers selected events consecutively starting at `seq: 0`, recomputes `run.totals` (`messages` and `tool_calls`) and `outcome` over the selected events, and records provenance at `run.slice_of`:
  ```json
  {
    "digest": "<source_tape_sha256>",
    "from": 1,
    "to": 10,
    "kinds": ["tool_call", "tool_result"],
    "events": 4
  }
  ```
- **Pairing integrity**: The resulting slice must itself pass `validate_tape`. Selecting `--kind tool_result` without `--kind tool_call` (or setting `--from` after a `tool_call` whose `tool_result` falls inside the window) fails validation (`answers no earlier tool_call in this tape`) and exits `1` without writing output.

### 2.10 `screen`

Audit a valid tape for unredacted POSIX absolute paths before sharing or archiving.

```sh
flighttape screen [-o OUTPUT] tape
```

- Walks every string leaf and object key inside `message.text`, `tool_result.content`, and `tool_call.input` iteratively (up to container depth 1,000).
- Emits a JSON report to `stdout` or `-o OUTPUT`:
  ```json
  {
    "tape": "a88422b41393c0f6be88b624ce7595c1615af3c8eb0af3ae3d15dda622080e79",
    "strings_screened": 6,
    "paths_found": 0,
    "hits": [],
    "clean": true
  }
  ```
  When hits occur, each entry in `hits` records `{"seq": <int>, "field": "text|content|input", "in": "key|value", "paths": <int>}`. **The report never echoes the matched string or any substring of a detected path.**
- **Exit code**: Exits `0` when `clean` is `true` (`paths_found == 0`), and exits `1` when `clean` is `false` (`paths_found > 0`) or on any read/validation/write failure. Note that when `-o OUTPUT` is supplied and `clean` is `false`, `screen` still writes the JSON report to `OUTPUT` and exits `1` so CI pipelines can both archive the report and fail the job.
- **Input protection and atomic write**: `screen -o` resolves both `tape` and `OUTPUT`, refuses symlink loops (`error, the output path cannot be resolved`), refuses to overwrite the input tape by path or Unix hardlink (`error, a screen report never overwrites the tape`), and writes via an exclusive mode `0600` sibling temporary file (`<name>.<8-hex-chars>`) renamed over `OUTPUT`.

---

## 3. Path Redaction and Screening Boundaries

`convert` and `screen` share the exact token-level path detector (`convert::redact_token` / `convert::paths_in`). Operators sharing tapes across trust boundaries must understand both what is caught and what is intentionally or structurally excluded:

| Token Shape | Example | Caught by `convert` / `screen`? | Notes |
| --- | --- | --- | --- |
| Bare multi-segment POSIX path | `/Users/alice/work/app.rs` | Yes (`app.rs`) | At least two non-empty `/` segments required |
| Quoted or bracketed path | `"/home/dev/a.txt"`, `(/tmp/a/b)` | Yes (`"a.txt"`, `(b)`) | Leading `"'`([{<` and trailing punctuation preserved |
| Key-value flag or assignment | `--file=/var/log/app.log`, `path:/a/b` | Yes (`--file=app.log`, `path:b`) | `[A-Za-z0-9_.-]*[=:]+` prefix qualifies |
| Markdown link destination | `[doc](/Users/alice/notes.md)` | Yes (`[doc](notes.md)`) | Stem ending in `]` before `(` qualifies |
| Single-segment root path | `/etc`, `/tmp` | No | Fewer than 2 non-empty segments; treated as root label |
| Relative path | `src/main.rs`, `./tests/a.rs` | No | Safe repo-relative paths stay intact |
| URL with `://` scheme | `https://example.com/a/b`, `file:///Users/alice/a.txt` | No | `://` scheme tokens are skipped; scrub `file://` URIs upstream |
| Double-slash / UNC prefix | `//server/share/file.txt` | No | Leading `//` advances past the slash pair |
| XML/HTML tag prefix | `<path>/Users/alice/file.txt</path>` | No | `<path>` stem before `/` lacks `=`, `:`, or `]` |
| Function-call prefix | `open(/Users/alice/file.txt)` | No | Identifier stem `open` before `(` lacks `=`, `:`, or `]` |
| Windows backslash path | `C:\Users\alice\file.txt` | No | Backslash separators are not inspected by the POSIX slash scanner |

---

## 4. Canonical Form 1 and Tape Digests

> Note: Integer `-0` normalizes to `0` during Canonical Form 1 serialization; all other integer tokens preserve exact base-10 digits within the digit limit.

Every `flighttape` digest (`digest`, `verify`, `compare`, `rollup`, `slice_of.digest`, `screen.tape`) is the lowercase hexadecimal SHA-256 of the value's UTF-8 bytes in **Canonical Form 1**:

1. **No insignificant whitespace**: Elements are separated by `,` (`0x2C`) and object members by `:` (`0x3A`) with zero spaces or newlines.
2. **UTF-16 code unit key sort**: Object keys are sorted ascending by their raw UTF-16 code units (`Vec<u16>` lexicographical order), matching ECMAScript / RFC 8785 (JCS) key ordering.
3. **Strings and surrogate refusal**: Control characters `U+0000..U+001F`, `"`, and `\` use standard JSON escapes (`\b`, `\t`, `\n`, `\f`, `\r`, `\"`, `\\`, or lowercase `\u00xx`); all other valid Unicode scalars are emitted as raw UTF-8. Any lone UTF-16 surrogate (`0xD800..0xDFFF`) cannot be encoded as UTF-8 and causes canonicalization to fail with `DigestError("the tape holds text that cannot be encoded")`.
4. **Exact integers**: JSON integer tokens (numbers without `.` or `e`/`E`) are stored as exact base-10 digit strings and emitted verbatim without floating-point coercion, up to **4,300 digits** (excluding optional leading `-`). Integers over 4,300 digits fail with `DigestError`.
5. **ECMAScript shortest round-trip floats**: JSON floating-point tokens are parsed as IEEE-754 `f64` and formatted identically to ECMAScript `Number.prototype.toString()` / RFC 8785:
   - `-0.0` and `+0.0` both render as `"0"`.
   - `NaN`, `+Infinity`, and `-Infinity` are refused with `DigestError("the tape holds a non-finite number")`.
   - Finite non-zero floats use exact big-integer decimal expansion to select the shortest round-trip digit string (with half-to-even tie-breaking and power-of-two asymmetric interval checking), formatted in fixed notation when `-6 < point <= 21` and scientific notation (`e+` / `e-`) otherwise. Note `1.0` parses as `Float(1.0)` and renders canonically as `"1"`.
6. **Container depth bound**: Arrays and objects nested deeper than **1,000** containers fail with `DigestError("the tape is nested beyond the interpreter limit")`.

---

## 5. Rust Library Embedding

Add `flighttape` to `Cargo.toml`:

```toml
[dependencies]
flighttape = "1.0"
```

Example: parsing, validating, screening, and computing the canonical digest of a tape in Rust:

```rust
use std::path::Path;
use flighttape::convert::convert_file;
use flighttape::identity::{canonical_bytes, tape_digest};
use flighttape::json::parse;
use flighttape::screen::screen_tape;
use flighttape::validate::{tape_duration, validate_tape};
use flighttape::value::Value;

fn inspect_session(jsonl_path: &Path) -> Result<String, String> {
    let tape = convert_file(jsonl_path).map_err(|e| e.to_string())?;

    let defects = validate_tape(&tape);
    if !defects.is_empty() {
        return Err(format!("invalid tape: {}", defects.join("; ")));
    }

    let report = screen_tape(&tape).map_err(|e| e.to_string())?;
    if report.get("clean") != Some(&Value::Bool(true)) {
        return Err("tape contains unredacted POSIX paths".to_string());
    }

    let _canonical = canonical_bytes(&tape).map_err(|e| e.to_string())?;
    let _seconds = tape_duration(&tape);
    let raw_json = flighttape::dumps::dumps_indent(&tape);
    let reparsed = parse(&raw_json).map_err(|_| "parse error".to_string())?;

    tape_digest(&reparsed).map_err(|e| e.to_string())
}
```

Key public modules:

- `flighttape::value` - `Value`, `Text` (UTF-16 code unit string preserving lone surrogates for lossless diagnostics), `object`, `set_key`, `int_add`, `int_cmp`.
- `flighttape::json` - Iterative JSON parser (`parse`) supporting RFC 8259 plus `NaN`/`Infinity` detection for downstream defect reporting.
- `flighttape::validate` - `validate_tape`, `tape_duration`, `EVENT_KINDS`, `OUTCOMES`, `BASES`.
- `flighttape::identity` - `canonical_bytes`, `tape_digest`, `render_number`, `DigestError`.
- `flighttape::convert` - `convert_file`, `convert_lines`, `redact_text`, `sanitize`, `paths_in`.
- `flighttape::compare`, `flighttape::rollup`, `flighttape::slice`, `flighttape::screen`, `flighttape::play`, `flighttape::dumps`.

---

## 6. Third-Party Harness Adapter Specification

Although `flighttape`'s built-in `convert` command currently reads Claude Code session JSONL (`claude-code-jsonl`), **any agent harness** (Codex CLI, Gemini CLI, OpenAI Responses/Chat Completions loops, Anthropic Messages loops, LangGraph, or custom orchestrators) can emit native `tape_version` `"1.0"` files consumed directly by `validate`, `play`, `digest`, `verify`, `compare`, `rollup`, `slice`, `screen`, and all downstream fleet tools.

### 6.1 Top-Level Document Shape

A compliant `1.0` tape is a UTF-8 JSON object (without BOM) containing four required top-level keys:

```json
{
  "tape_version": "1.0",
  "run": {
    "harness": "codex-cli",
    "session_id": "run-20261127-01",
    "source_format": "codex-rollout-jsonl",
    "started_at": "2026-11-27T10:00:00.000Z",
    "skipped_lines": {},
    "redacted_values": 0,
    "totals": {
      "input_tokens": 1240,
      "output_tokens": 310,
      "cache_creation_input_tokens": 0,
      "cache_read_input_tokens": 512,
      "tool_calls": 1,
      "messages": 3
    }
  },
  "events": [
    {
      "seq": 0,
      "kind": "message",
      "sidechain": false,
      "timestamp": "2026-11-27T10:00:00.000Z",
      "role": "user",
      "text": "Run the test suite and summarize failures."
    },
    {
      "seq": 1,
      "kind": "tool_call",
      "sidechain": false,
      "timestamp": "2026-11-27T10:00:02.150Z",
      "call_id": "call_01",
      "tool": "shell",
      "input": {
        "cmd": "cargo test"
      }
    },
    {
      "seq": 2,
      "kind": "tool_result",
      "sidechain": false,
      "timestamp": "2026-11-27T10:00:06.800Z",
      "call_id": "call_01",
      "is_error": false,
      "content": "test result: ok. 42 passed; 0 failed"
    },
    {
      "seq": 3,
      "kind": "message",
      "sidechain": false,
      "timestamp": "2026-11-27T10:00:08.000Z",
      "role": "assistant",
      "text": "All 42 tests passed."
    }
  ],
  "outcome": {
    "status": "completed",
    "basis": "final-assistant-message"
  }
}
```

### 6.2 `run` Object Rules

| Field | Required | Type & Constraints |
| --- | --- | --- |
| `harness` | Yes | Non-empty string identifying the agent runtime (e.g. `"claude-code"`, `"codex-cli"`, `"gemini-cli"`). |
| `session_id` | Yes | Non-empty string uniquely identifying the session (`"unknown"` if absent in raw source). |
| `source_format` | No | String (or `null`) naming the raw log format (e.g. `"openai-responses-jsonl"`). |
| `started_at` | No | String (or `null`), typically the ISO-8601 timestamp of the first observed record. |
| `skipped_lines` | No | Object mapping skip-reason strings to non-negative integers (`>= 0`). |
| `totals` | No | Object mapping counter names to non-negative integers (`>= 0`). Standard keys: `input_tokens`, `output_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`, `tool_calls`, `messages`. |
| `redacted_values` | No | Non-negative integer counting path redactions applied during conversion. |
| `slice_of` | No | Provenance object added by `flighttape slice` (`digest`, `from`, `to`, `kinds`, `events`). |

### 6.3 `events` Array Invariants

`events` must be a non-empty JSON array (`minItems: 1`). Every element must be an object satisfying:

1. **Consecutive `seq`**: `events[i].seq` must be the exact integer `i` (`0, 1, 2, ...` with no gaps, duplicates, floats, or booleans).
2. **Valid `kind`**: Must be one of the 8 event kinds:
   - `"session_start"`
   - `"message"` (carries `"role": "user" | "assistant" | ...` and `"text": "<string>"`)
   - `"tool_call"` (requires non-empty string `"call_id"`, non-empty string `"tool"`, and optional `"input"` object/value)
   - `"tool_result"` (requires `"call_id"` matching an **earlier** `"tool_call"` in the same `events` array, optional boolean `"is_error"`, and optional string `"content"`)
   - `"diff"`
   - `"spend"`
   - `"note"`
   - `"session_end"`
3. **Explicit `sidechain`**: Boolean (`true` for subagent/sidechain branches, `false` for main thread).
4. **Monotonic ISO-8601 `timestamp`** (when present and non-null): Must be a full ISO-8601 date-time string with an explicit timezone (`Z` or `+HH:MM` / `-HH:MM`, e.g. `"2026-11-27T10:00:02.150Z"`). Across all events carrying a timestamp, each timestamp must be `>=` all preceding event timestamps (`events[i].timestamp` may never run backward).
5. **Unique `tool_call.call_id` and causal ordering**: Every `tool_call` in a tape must carry a distinct non-empty `call_id`. Every `tool_result` must reference a `call_id` that already appeared on a `tool_call` at a strictly smaller `seq`. Multiple `tool_result` events may reference the same earlier `tool_call` if a harness streams partial results, provided the `tool_call` comes first.
6. **Finite numbers and valid Unicode**: No number anywhere in the tape may be `NaN` or `Infinity`, no integer may exceed 4,300 digits, and no string may contain unpaired UTF-16 surrogates.

### 6.4 `outcome` Matrix and Derivation Rules

`outcome` is a required object with a required `status` and optional `basis`. When `basis` is present, `validate_tape` enforces exact pairing with `status`:

| `outcome.status` | Allowed `outcome.basis` | Reference `determine_outcome` Trigger |
| --- | --- | --- |
| `"completed"` | `"final-assistant-message"` | Last non-sidechain event is a `message` with `role == "assistant"`. |
| `"failed"` | `"final-error-result"` | Last non-sidechain event is a `tool_result` with `is_error == true`. |
| `"aborted"` | `"user-interrupt-marker"` | Last non-sidechain event is a `message` with `role == "user"` whose trimmed text equals `"[Request interrupted by user]"` or `"[Request interrupted by user for tool use]"`. |
| `"unknown"` | `"no-events"` or `"unresolved-ending"` | `"no-events"` when zero non-sidechain events exist; `"unresolved-ending"` for any other final non-sidechain event (such as a regular user prompt or unanswered `tool_call`). |

### 6.5 Adapter Acceptance Checklist

Before shipping a custom harness adapter, run these four commands over its generated tapes:

```sh
# 1. Structural, pairing, timestamp, and finiteness check (must exit 0)
flighttape validate candidate.tape.json

# 2. POSIX absolute-path leak audit (must exit 0 with "clean": true)
flighttape screen candidate.tape.json

# 3. Canonical Form 1 digest check (must print 64 hex chars and exit 0)
flighttape digest candidate.tape.json

# 4. Human timeline inspection (must exit 0)
flighttape play candidate.tape.json
```

---

## 7. Shell Completions & `cargo-binstall`

### Fast Binary Install via `cargo-binstall`

If you use [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall), `Cargo.toml` carries `[package.metadata.binstall]` so prebuilt signed GitHub Release archives install without local compilation:

```sh
cargo binstall flighttape
```

### Shell Tab Completion

Static zero-dependency completion scripts ship under `completions/` (and inside every release archive):

```sh
# Bash
source completions/flighttape.bash

# Zsh (place _flighttape on your $fpath)
fpath=(./completions $fpath) && autoload -Uz compinit && compinit

# Fish
cp completions/flighttape.fish ~/.config/fish/completions/

# PowerShell
. ./completions/flighttape.ps1
```

