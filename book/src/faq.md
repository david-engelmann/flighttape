# Flighttape Frequently Asked Questions

### Does `flighttape` upload transcripts anywhere?
Never. `flighttape` has zero network dependencies (`sha2` is its sole crate dependency) and performs strictly local filesystem and standard-stream I/O.

### Does `flighttape screen` catch API keys or bearer tokens?
No, by design. `convert` redacts multi-segment POSIX and Windows filesystem paths (`<REDACTED>`), and `screen` checks string leaves for residual filesystem paths (`paths_found`). Neither command is a secret scanner. Run a dedicated secret scanner (such as `gitleaks` or `trufflehog`) alongside `flighttape screen` before publishing tapes from environments that handle credentials.

### Why did `flighttape rollup` exit `1` after `flighttape convert-dir`?
`convert-dir` writes `summary.json` beside the converted `.tape.json` files in its output directory. When `rollup` scans a directory, any file that is not a valid `1.0` flight tape (including `summary.json`) is listed in `excluded[]` and triggers exit code `1` so automated pipelines never silently ignore corrupt or foreign files. Write `convert-dir` outputs to a staging folder or move `summary.json` outside the pure tape directory before running `rollup`.

### How do I integrate another agent harness (Codex, Gemini CLI, Cursor, OpenCode)?
Emit JSON conforming to `schema/tape.schema.json` (documented step-by-step in [GUIDE.md Section 6](GUIDE.md#6-third-party-harness-adapter-specification)) and verify every emitted bundle with `flighttape validate <file>`. Once validated, `play`, `compare`, `rollup`, `slice`, `screen`, `secondtake`, `unheard`, and `retroforge` work identically regardless of which harness produced the tape.

### Can `flighttape` overwrite a session or tape in place with `-o`?
No. `flighttape` resolves `-o` against its input path (following symlinks and hardlinks) and exits `1` with `error, ...` rather than clobbering an input artifact. Valid writes go to a sibling temporary file and atomically rename into place.
