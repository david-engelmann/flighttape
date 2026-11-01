# flighttape Contract

This Contract is the executable spec for flighttape. Every obligation
is checked by a named test or by the validator itself. The Contract is
reported as met or not met in the README status section.

## The bundle

- FT-1. A tape is one JSON object with tape_version "1.0", a run
  object, an events list, and an outcome object. The published schema
  at schema/tape.schema.json states the shape. The validator
  enforces the schema, and also the structural rules JSON Schema
  cannot state, namely consecutive seq values, tool call pairing,
  and chronological timestamp order. A conformance test loads the
  schema and checks that the two agree on the shared vocabulary.
  Test, tests/test_validate.py and tests/test_schema.py.
- FT-2. The validator rejects a malformed tape and names every defect
  it finds in one pass, never just the first. Test,
  tests/test_validate.py test_all_defects_are_named_in_one_pass.

## Events

- FT-3. Events carry a seq integer starting at 0 with no gaps and no
  repeats, a kind, and, when the source carries one, a timestamp.
  Timestamps never run backward inside one tape. Test,
  tests/test_validate.py.
- FT-4. A tool_result event names the tool_call it answers through
  call_id, and that call must appear earlier in the same tape. A
  result with no matching earlier call is a defect. Test,
  tests/test_validate.py and tests/test_convert_claude.py.
- FT-5. Sidechain lines that carry convertible content become
  events flagged sidechain true, never merged into the main chain.
  Sidechain content the converter cannot turn into an event is
  counted in run.skipped_lines like any other loss. Test,
  tests/test_convert_claude.py.

## Conversion

- FT-6. The Claude Code converter reads a session JSONL file and
  emits one tape. User and assistant text become message events,
  tool_use blocks become tool_call events keyed by the source tool
  id, tool_result blocks become tool_result events carrying that id,
  and token usage on assistant lines is summed into the run totals,
  cache creation and cache read tokens included under their own
  keys. Tool result content is carried into the tape as text. Test,
  tests/test_convert_claude.py.
- FT-7. Lines the converter does not turn into events are counted in
  run.skipped_lines, by reason, covering non-message line types,
  unparseable and non-object lines, malformed message and usage
  fields, unknown and non-object content blocks, contentless lines,
  and lines from a second session, so a conversion can be audited
  for silent loss. Malformed input is counted, never fatal, and an
  empty conversion yields zero events and fails validation rather
  than gaining a fabricated event. Test,
  tests/test_convert_claude.py.
- FT-8. The converter records the source format name and the session
  id only. It never writes the source file path, the working
  directory, or the branch name into a tape, and absolute paths
  inside tool inputs are reduced to their final component and
  counted in run.redacted_values. Test,
  tests/test_convert_claude.py test_tape_carries_no_source_paths,
  whose fixture lines carry a working directory and a branch so the
  test can fail on what it names.

## Fixtures

- FT-9. Every fixture committed to this repo is synthetic, written
  for this repo. Real session conversions run locally only and are
  reported as numbers only, never committed.

## Playback, Round 2 obligations, written before the code

- FT-10. A tape can be played back as a readable timeline with the
  play command. The rendering is a pure function of the tape, the
  same tape always renders byte identical output, and a golden
  file test holds the rendering to a stored expected output.
  Test, tests/test_play.py.
- FT-11. The timeline header states the harness, the session id,
  the outcome status, the event counts taken from the events
  themselves, and every recorded token total, zero totals included
  when the key is recorded. A second header line states the start
  time, the source format, the skipped line counts, and the
  redaction count when the tape carries them. Every event renders
  as one entry carrying its seq, its offset in seconds from the
  first timestamped event when timestamps exist, its kind, and its
  content, with sidechain events marked as sidechain. Offsets under
  ten seconds render to two decimals so sub second spacing stays
  visible. Test, tests/test_play.py.
- FT-12. Playback loses nothing the tape carries. Message text,
  tool names, tool call inputs, tool result content, error flags,
  the content of diff, spend, and note events, and any further
  fields an event carries, rendered as a sorted JSON line, all
  appear in the rendering. A test renders the synthetic fixture
  tape and asserts each carried field appears, and probe tapes
  cover the diff, spend, cache total, and extra field cases. Test,
  tests/test_play.py.
- FT-13. Playing a malformed tape fails through the validator
  first, with the defects named and a non-zero exit, never a
  partial rendering. A file that is not valid JSON at all fails the
  same way, with a named defect and no traceback. Test,
  tests/test_cli.py.

## Amendments, Round 1, 2026-10-31

The Round 1 reviews found the original wording of FT-1, FT-5, FT-6,
FT-7, and FT-8 claimed more than the code did. The obligations above
were narrowed or extended to state exactly what the code now does,
and the code was fixed to meet them. The full record is the Round 1
dossier in the goal files of the portfolio programme. One known
sequencing deviation stands, the full Contract wording and the first
slice code landed in one commit, because the stub Contract from the
setup commit was the sequencing the programme required and the
extension happened in the same working pass. It is recorded here
rather than hidden.

## Amendments, Round 2, 2026-10-31

The Round 2 reviews found FT-11 and FT-12 claimed more than the
renderer did. Tool inputs never rendered, diff and spend content
was dropped, recorded zero totals were hidden, countable header
figures trusted the totals over the events, and offsets hid sub
second spacing. The obligations above were extended to state what
the code now does, and the code was fixed to meet them. The full
record is the Round 2 dossier in the goal files of the portfolio
programme.

## Outcome determination, Round 3 obligations, written before the code

- FT-14. The converter determines the outcome from the tape's main
  chain events, never a constant. Main chain means events not
  flagged sidechain. Trailing session_end events are set aside
  first, because they mark the end of the recording, not the end
  of the run. The rules then read the final main chain event.
  A user message whose text begins with the interrupt marker
  [Request interrupted by user gives aborted. An error tool result
  gives failed. An assistant message gives completed. Anything
  else, a tool call, a tool result without error, a user message
  without the marker, or any other kind, gives unknown, and a tape
  with no main chain events gives unknown. Test,
  tests/test_outcome.py and tests/test_convert_claude.py.
- FT-15. The outcome object records the basis, the name of the
  rule that fired, alongside the status, so the determination can
  be audited without rerunning it. The basis names are
  final-assistant-message for completed, final-error-result for
  failed, user-interrupt-marker for aborted, and no-events or
  unresolved-ending for unknown. The schema states the basis
  field, and the validator rejects a basis that is present but not
  a non-empty string. Test, tests/test_outcome.py,
  tests/test_validate.py, and tests/test_schema.py.
- FT-16. The determination is a pure function of the events list,
  exposed as determine_outcome in its own module and used by the
  converter. The same events always give the same outcome, with
  no clock, no file access, and no mutation of the input. Test,
  tests/test_outcome.py.
- FT-17. The playback header states the outcome basis in
  parentheses after the status when the tape carries one, so the
  evidence for the status travels with the rendering. The golden
  file changes only in the header outcome segment, and the change
  is re-verified line by line before it is kept. Test,
  tests/test_play.py.

## Amendments, Round 3, 2026-11-01

The Round 3 reviews found FT-15's basis was enforced as any
non-empty string, so a tape could carry a basis naming no rule,
or a basis contradicting its status, and still validate. FT-15
is extended, the basis must be one of the five named values and
must pair with the status as the FT-15 mapping states, the
schema states the five as an enum, and the validator enforces
the vocabulary and the pairing. Whether a basis is truthful
against the events stays the producer's duty, as with the run
totals. FT-14 is clarified in two places. A tape left with no
main chain events after the session_end set aside gives unknown
with basis no-events, covering the empty tape, the all sidechain
tape, and the session_end only tape with one reading. The
sidechain and is_error flags are booleans, is_error is typed
boolean in the schema and checked by the validator when
present, and determine_outcome reads only the value True as
set. FT-16 is clarified, determine_outcome takes a list and
raises TypeError on anything else, and non-object entries in
the list are ignored. The full record is the Round 3 dossier
in the goal files of the portfolio programme.
