# Security Policy

## Supported Versions

Security fixes land on `main` and the latest tagged `1.x` release.

## Security Guarantees and Non-Goals

1. **Untrusted session and tape inputs**: During `flighttape convert`, malformed, non-UTF-8, or over-depth session lines are skipped and tallied by reason in `run.skipped_lines` rather than crashing conversion. During `validate`, `verify`, `play`, `compare`, `rollup`, `slice`, and `screen`, malformed tapes produce deterministic named errors (`exit 1`) on `stderr` rather than panics, protected by a `1,000`-level nesting cap and a `512 MiB` worker stack.
2. **Host path redaction vs secrets**: `convert` redacts multi-segment POSIX and Windows host paths (`<REDACTED>`), and `screen` audits all string leaves for residual paths. Neither command scans for API keys or tokens; pair `flighttape screen` with a secret scanner before external sharing.
3. **Atomic report output (`screen -o`)**: `flighttape screen -o` refuses overwriting its input tape and writes through a sibling temporary file (`0600` permissions on Unix; platform default ACLs on non-Unix) before atomic rename. Choose distinct output paths for `convert -o` and `slice -o`.

## Reporting a Vulnerability

Report security vulnerabilities privately via [GitHub Security Advisories](https://github.com/david-engelmann/flighttape/security/advisories/new).
