# Ecosystem Comparison

How `flighttape` compares to adjacent agent-tracing and observability approaches.

| Dimension | `flighttape` | SaaS LLM Observability (LangSmith, Langfuse, Braintrust) | OpenTelemetry GenAI Spans | Raw Harness Transcripts (`~/.claude/projects/*.jsonl`) |
| :--- | :--- | :--- | :--- | :--- |
| **Deployment model** | Single static Rust binary (`~972 KiB`), 100% local & offline | Hosted cloud or multi-container self-hosted stack (Postgres/ClickHouse) | Collector + backend required to query or diff | Local files, undocumented & harness-specific |
| **Content addressing** | Canonical Form 1 SHA-256 `tape_digest` (`verify` in `<20 ms`) | Mutable database rows keyed by UUID | Trace/Span IDs without canonical payload hash | None; raw append logs |
| **Privacy & path redaction** | Automatic path redaction on `convert` + `screen` CI gate | SDK callbacks required before network egress | Manual processor configuration | Raw host paths, usernames, and env leaks |
| **Cross-tool portability** | Pinned `1.0` JSON Schema consumed by `secondtake`, `unheard`, `retroforge` | Proprietary API export per vendor | Span attributes still evolving across drafts | Breaks whenever harness changes internal JSONL |
| **Runtime dependencies** | 1 crate (`sha2`), zero network calls | Python/TS SDKs, async background exporters | OTel SDK + protobuf/HTTP transport | None |

## Sibling Boundary Inside the Retrospective Line

- **`flighttape` vs `onrecord`**: `flighttape` records **one finished session run** (events, tool calls, token spend, outcome). `onrecord` remembers **claims and receipts across runs**, linking assertions into linear supersession chains and surfacing cross-agent conflicts.
- **`flighttape` vs `secondtake`**: `flighttape compare` diffs **two individual tapes** event-by-event. `secondtake compare` evaluates **two directories of tapes** (`before/` vs `after/`) matched by task key against pre-registered regression margins to issue a `keep`, `tune`, `revert`, or `abstain` verdict.
