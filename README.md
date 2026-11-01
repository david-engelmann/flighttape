# flighttape

Every agent run from every harness lands as one flight tape holding events, tool calls, diffs, spend, timings, and the outcome, in a bundle any tool can read back.

## What this is

Each harness keeps its own books in its own dialect, and the shared standards for agent traces are still experimental, so the history of a mixed fleet is unreadable as one record. Flighttape defines one bundle format for a finished run and ships converters that turn each harness's session files into tapes. Any tool that needs to study past runs reads tapes instead of reverse engineering every harness on its own. The boundary with its siblings is one sentence. Flighttape records the run, onrecord remembers across runs, badbatch stamps what the run left behind.

## Status

Seeded in October 2026. The first slice is built locally on the first slice branch, a tape schema, a Claude Code converter, and a validator.
The remote seeding pass is waiting on a token access fix, so work is
banked locally until pushes clear. Part of the impatient programming
portfolio, in the Retrospective arc.

## Programme status

- Closed perfection rounds, 2 of 100. A round counts only when a full
  cycle of work, audit, review, red team, and approval has closed.
- Contract, FT-1 to FT-9 as amended in Round 1, met. The Round 1
  reviews and their findings are recorded with the programme. The
  slice done criterion, a week of real lane sessions converting and
  validating, is still outstanding and is not claimed here, the
  converter ships a convert-dir command so a session set can be
  converted in one run when the sessions exist.
- Round 2 adds playback, the play command renders a valid tape as
  a readable timeline, held to a golden file. Obligations FT-10 to
  FT-13 as amended in Round 2, met, with the reviews recorded with
  the programme. Malformed tapes refuse to play, by FT-13 design.
- Round 3 adds outcome determination, the converter derives the
  outcome status from the tape's own events and records the basis,
  the rule that fired, beside it, instead of stamping every tape
  unknown. Obligations FT-14 to FT-17, recorded with the programme.
- Local gate, python3 scripts/codepoint_scan.py and, once tests exist,
  python3 -m pytest tests -q. CI runs the same commands.

## Licence

MIT. See LICENSE.
