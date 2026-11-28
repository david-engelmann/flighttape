# Flighttape Frequently Asked Questions

### Does `flighttape` upload transcripts anywhere?
Never. `flighttape` has zero network dependencies (`sha2` is its sole crate dependency) and performs strictly local filesystem and standard-stream I/O.

### Does `flighttape screen` catch API keys or bearer tokens?
No, by design. `convert` redacts multi-segment POSIX and Windows filesystem paths (`<REDACTED>`), and `screen` checks string leaves for residual filesystem paths (`paths_found`). Run a dedicated secret scanner alongside `flighttape screen` before sharing tapes from environments that handle credentials.

### Which files does `flighttape rollup` inspect in a directory?
`rollup` inspects top-level `.json` entries in the target directory (ignoring non-`.json` files such as `.jsonl` or `.txt`). Any `.json` file that is not a valid `1.0` flight tape (for example `summary.json` written by `convert-dir`) is reported in `excluded[]` and causes `rollup` to exit `1` so automated pipelines never silently skip non-tape JSON files.

### Which commands enforce atomic sibling writes on `-o`?
`flighttape screen -o <path>` enforces input-preservation and writes atomically through a sibling temporary file (`0600` mode on Unix). For `convert` and `slice`, always specify a distinct `-o` destination path separate from the source file.
