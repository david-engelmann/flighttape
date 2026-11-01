# flighttape Contract, stub

The full Contract is the first build act of phase 1. This stub records the
obligations the setup already knows it owes. Each obligation becomes
executable before any feature code that depends on it lands.

- FT-1. A tape is one JSON bundle holding run metadata, an ordered event
  list, and an outcome, valid against the published tape schema.
- FT-2. A validator rejects a malformed tape and names every defect it
  finds, never just the first.
- FT-3. The Claude Code converter preserves sidechain events and tool
  call identity, so no part of a run is silently dropped or re-keyed.
- FT-4. Every fixture committed to this repo is synthetic. Real session
  conversions run locally only and are reported as numbers only.
