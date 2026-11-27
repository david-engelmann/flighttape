# Security Policy

## Supported Versions

| Version | Supported |
| --- | --- |
| 1.0.x | Yes |
| < 1.0.0 | No |

## Reporting a Vulnerability

Do not open a public GitHub issue for suspected security vulnerabilities.

Report vulnerabilities privately through [GitHub Security Advisories](https://github.com/david-engelmann/flighttape/security/advisories/new) on `david-engelmann/flighttape`. Include:

1. Affected `flighttape` version or commit SHA and operating system.
2. Minimal reproduction steps or a synthetic input demonstrating the issue.
3. Expected behavior versus observed behavior and potential impact.

Reports receive an initial acknowledgment within 72 hours. Once a fix is validated, a patched release and advisory will be published together.

## Security Model and Operational Boundaries

- **Untrusted session and tape inputs**: `flighttape` parses raw JSON and JSONL inputs without executing embedded content. Container depth is capped at 1,000 levels in canonical rendering and conversion, integer canonicalization is bounded at 4,300 digits, and malformed lines or tapes produce named errors rather than panics.
- **Path redaction and screening scope**: `convert` and `screen` inspect `message.text`, `tool_result.content`, and `tool_call.input` for multi-segment POSIX absolute paths (`/a/b...`), reducing them to their final path component during conversion and reporting counts (never path contents) during screening.
- **Out-of-scope secrets and path shapes**: `screen` is a POSIX path check, not a general secret scanner. API keys, bearer tokens, `file://` URIs, Windows drive paths (`C:\Users\...`), single-segment root paths (`/etc`), and unseparated prefixes such as `fn(/a/b)` or `<tag>/a/b` are not redacted by the POSIX token rule. Scrub credentials and non-POSIX paths upstream before publishing tapes externally.
- **Atomic report output**: `flighttape screen -o` refuses to overwrite its input tape (including hardlink aliases on Unix) and writes reports via an exclusive `0600` sibling temporary file before renaming into place.
