# Flighttape Wire Formats & Exit Contracts

All formats emitted by `flighttape` are UTF-8 JSON documents with deterministic field ordering and explicit exit codes.

## 1. The `1.0` Flight Tape Bundle (`schema/tape.schema.json`)

Every valid flight tape is a top-level JSON object carrying five required members:

```json
{
  "tape_version": "1.0",
  "tape_digest": "a88422b41393c0f6be88b624ce7595c1615af3c8eb0af3ae3d15dda622080e79",
  "run": {
    "run_id": "readme-example-001",
    "harness": "claude-code",
    "source_format": "claude-code-jsonl",
    "started_at": "2026-02-02T10:00:00.000Z",
    "ended_at": "2026-02-02T10:00:20.000Z",
    "duration_seconds": 20.0,
    "totals": {
      "input_tokens": 135,
      "output_tokens": 40,
      "cache_creation_input_tokens": 0,
      "cache_read_input_tokens": 0,
      "tool_calls": 1,
      "messages": 3
    },
    "redacted_values": 0,
    "skipped_lines": {}
  },
  "events": [
    {
      "seq": 0,
      "timestamp": "2026-02-02T10:00:00.000Z",
      "kind": "message",
      "role": "user",
      "sidechain": false,
      "text": "Synthetic prompt, sort the example records by name."
    }
  ],
  "outcome": {
    "status": "completed",
    "basis": "final-assistant-message"
  }
}
```

### Core Structural Invariants

1. **`tape_version`**: Exact string `"1.0"`.
2. **`events` sequence**: Contiguous `0..N-1` integer `seq` values; non-decreasing ISO-8601 UTC timestamps; every `tool_result` pairs with an earlier `tool_call` `call_id`.
3. **`outcome.status`**: Closed enum (`completed`, `failed`, `abandoned`, `unknown`) paired with an explicit `basis` string explaining how completion was inferred.
4. **`tape_digest`**: Lowercase 64-hex SHA-256 over the Canonical Form 1 byte representation of the root object with `tape_digest` removed.

## 2. Analytical Output Documents

- **`flighttape compare <a> <b>`**: JSON object containing `digests`, `outcome` (`a`, `b`, `changed`), `duration_seconds` (`a`, `b`, `delta`), `measures` deltas across all six counters, `redacted_values`, `skipped_lines`, and `events` (`count`, `by_kind`, `changed`, `added`, `removed`).
- **`flighttape rollup <dir>`**: JSON summary listing sorted `tapes[]`, `excluded[]` non-tape/invalid files with reasons, aggregate `counts`, `outcomes`, `totals`, and `duration_seconds`. Exits `0` when `excluded` is empty, `1` when any file in `<dir>` was excluded.
- **`flighttape screen <tape>`**: JSON privacy audit (`tape`, `strings_screened`, `paths_found`, `hits[]`, `clean`). Exits `0` when `clean == true`, `1` when `paths_found > 0`.

## 3. Exit Code Matrix

| Exit Code | Meaning | Typical Triggers |
| :--- | :--- | :--- |
| `0` | Clean pass | Valid tape, matching digest, zero excluded rollup files, `screen` found `0` paths |
| `1` | Domain refusal or check failure | Validation defect, digest mismatch, `-o` points at input file, `rollup` excluded files, `screen` detected host paths |
| `2` | CLI usage error | Unknown flag, missing positional argument, invalid integer on `--from` / `--to` |
