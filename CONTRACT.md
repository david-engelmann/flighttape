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

## Robustness, Round 4 obligations, written before the code

- FT-18. The converter never raises on its input. Any line,
  parseable or not, deeply nested or not, is converted whole or
  counted whole in run.skipped_lines, never partially emitted and
  never fatal. The counted reasons include too-deep for input
  nested beyond the interpreter limit and undecodable for lines
  that are not valid UTF-8, and a session file is read as bytes
  so undecodable lines reach the counter instead of raising.
  Test, tests/test_robustness.py.
- FT-19. The validator never raises on any value json.loads can
  produce. Values of the wrong type, including unhashable ones,
  in the fields the validator compares against its vocabularies
  and sets, the outcome status and basis, the event kind, and a
  tool result's call_id, are named as defects, never raised as
  exceptions. Test, tests/test_robustness.py.
- FT-20. A deterministic robustness corpus ships in the test
  suite, a seeded generator that mutates the synthetic fixture
  with character flips, truncations, and token substitutions
  from a fixed table, its case count pinned, plus a fixed table
  of adversarial tapes for the validator. Every corpus case runs
  the converter and the validator with no exception escaping,
  and every tape the converter produces is then validated the
  same way. Test, tests/test_robustness.py.
- FT-21. The command line inherits the guarantee. Validating or
  playing a tape file that cannot be read, is not valid UTF-8,
  or is not valid JSON, and converting a source that does not
  exist, each fail with a named error and exit 1, never a
  traceback. Test, tests/test_robustness.py and
  tests/test_cli.py.

Round 4 amendments, recorded during the repair that followed the
round's three reviews. A line whose type field is present but not
a string is counted as malformed:type under FT-18, a string type
keeps its own name as the reason as before, and a missing type, a
null type, or an empty string type is counted as missing-type;
an entry handed to convert_lines that is not a string at all is
counted as unparseable. The corpus generator of FT-20 mutates with six
moves, truncations, character flips, token substitutions from a
fixed table, prefix duplications, brace removals, and
whole-value type substitutions from a fixed table, the last of
which is what lets the corpus reach a structured value in a
string field. Under FT-21, convert-dir names any entry matching
the session pattern that is not a file, counts it as failed, and
carries on with the rest, and a failure to write an output file
or to create the output directory fails with a named error and
exit 1, so the no-traceback guarantee covers the write side as
the command line's docstring states.

## Redaction completeness, Round 5 obligations, written before the code

- FT-22. Every string the converter emits as text passes through
  one redactor before it enters the tape, message text, tool
  result content, and every string leaf of a tool call's input
  at any depth. Identifiers (the session id, call ids, tool
  names), timestamps, and the run's bookkeeping fields are not
  text and are outside this rule. Test, tests/test_redaction.py.
- FT-23. Inside a text string, an absolute path is a run of
  non-whitespace characters that begins with a slash where the
  characters before the slash in the same whitespace-delimited
  token are only opening punctuation (a quote, an apostrophe, a
  backtick, an opening bracket) or an identifier ending in an
  equals sign or a colon, that contains at least one further
  slash between non-empty components, that does not contain a
  scheme separator (://), and that does not begin with two
  slashes. Each such path is replaced by its final component,
  trailing slashes ignored, and any sentence punctuation after
  the path is preserved. A whole string that is itself such a
  path is the special case FT-8 has always required, and this
  rule refines FT-8 in one stated way, a lone first component
  such as /etc is not a path under this rule and is left alone.
  URLs and relative paths are not paths under this rule and are
  never altered. Test, tests/test_redaction.py.
- FT-24. run.redacted_values equals the number of paths the
  redactor replaced across the whole session, in text fields and
  in tool inputs alike, so the count reconciles against the tape
  a reader holds. Test, tests/test_redaction.py.
- FT-25. Text that contains no absolute path under FT-23 is
  emitted byte-identically to the pre-round converter, and the
  tape produced from the synthetic fixture is unchanged. Test,
  tests/test_redaction.py and the pre-existing suites.

Round 5 amendments, recorded during the repair that followed the
round's three reviews. Under FT-22, the string keys of a tool
input object are redacted exactly like its values, and two keys
that redact to the same string merge into one entry when the
object is rebuilt, the later value winning. FT-23 as implemented
scans every slash in a token rather than only the first, and the
prefix that qualifies a path is local, measured from the
previous slash, a glue separator, or the end of the previous
replaced path in the same token. The qualifying forms are an
empty local prefix, an identifier ending in one or more equals
signs or colons, and an opening punctuation run that follows
the token start, such an identifier stem, or a closing bracket
(the markdown link form). A candidate path ends at a comma, a
semicolon, a question mark, a hash, an ampersand, or a colon
followed by a slash, the scheme exclusion judges the candidate
alone, and a double slash pair never starts a path and is
passed over whole, slashes in a run included. Two shapes stay
outside the rule by decision, a greater-than sign before a slash
(the redirect and markup forms), because the same character is
a comparison operator in prose, and a word ending in an opening
bracket (the function call form).

## Tape identity, Round 6 obligations, written before the code

- FT-26. A tape's canonical form, named canonical form 1 of the
  tape format, is defined as follows. Objects render with their
  keys sorted by Unicode code point, arrays in order, no
  whitespace anywhere, strings as JSON strings with non-ASCII
  characters written unescaped, the whole encoded as UTF-8.
  Integers render in decimal exactly. A finite floating point
  number renders as the ECMAScript number to string algorithm
  renders it, the rendering the JSON canonicalization scheme
  uses, so 2.0 renders as 2 and 1e-7 renders as 1e-7. The
  literals true, false, and null render as themselves. Changing
  this form breaks every citation made under it and counts as a
  breaking change on par with a tape format version change.
  Test, tests/test_identity.py.
- FT-27. A tape's digest is the SHA-256 of its canonical form,
  written as lowercase hex. The digest depends only on the
  tape's content, two files that parse to the same tape, in any
  key order and with any whitespace, digest identically. The
  digest is recorded outside the tape by whoever cites it and
  is never stored inside the tape it identifies. The digest of
  the synthetic fixture tape is pinned in the test suite, so a
  silent change to the canonical form fails loudly. Test,
  tests/test_identity.py.
- FT-28. A tape that cannot be canonicalized is refused with a
  named DigestError, never a bare exception, when it holds a
  non-finite number anywhere (JSON cannot carry one portably),
  an object key that is not a string, or nesting beyond the
  interpreter limit. The digest identifies content, not
  validity, a tape that fails validation still digests. Test,
  tests/test_identity.py.
- FT-29. The validator names every non-finite number in a tape
  as a located defect, walking the whole value iteratively so
  the check cannot exhaust the stack or loop on a cyclic
  structure, holding FT-19. Test, tests/test_identity.py and
  tests/test_validate.py.
- FT-30. The command line gains digest and verify. digest
  prints the tape's digest and nothing else. verify takes a
  tape and an expected digest, exiting 0 when they match and 1
  with a named mismatch when they do not. Both commands share
  the tape loader's named errors (FT-21) and fail with a named
  error and exit 1 when the tape cannot be digested. Test,
  tests/test_identity.py.

Amendments recorded in the Round 6 repair. FT-26, keys are
sorted by their UTF-16 code units, the ordering the JSON
canonicalization scheme uses, not by Unicode code point as
first written. The two orders agree inside the basic
multilingual plane and diverge outside it, and a citation a
verifier in another language checks must follow the scheme.
FT-28, the refusal classes also cover text that cannot be
encoded, an unpaired surrogate being the reachable case, since
a JSON escape can carry one into a loaded tape, and a value
the canonical form cannot render, an integer beyond the
rendering limit being the reachable case. FT-30, verify
accepts the expected digest in any letter case and ignores
whitespace around it.

## Tape comparison, Round 7 obligations, written before the code

- FT-31. The compare command takes two tape files. Both are
  loaded with the shared loader (FT-21) and both must
  validate; a side that fails validation is named with its
  defects and the command exits 1 without producing a
  report, the refusal play makes (FT-13). Comparing two
  valid tapes always exits 0; differences are data in the
  report, never a failure. Test, tests/test_compare.py.
- FT-32. The report carries both tapes' digests (FT-27), each
  side's outcome status and basis and whether they differ,
  each side's duration in seconds and the delta when both
  are known (duration is the span between the first and last
  parseable event timestamps, null for a side with fewer than
  two), and a measure for every numeric entry in either
  side's run totals, plus redacted_values and each
  skipped_lines reason, giving each side's value and the
  delta when both sides carry a number for it. Test,
  tests/test_compare.py.
- FT-33. Events are matched by seq. The report counts events
  per side, counts them by kind per side, and lists the seqs
  whose events differ between the sides (changed, judged by
  the events' canonical forms under FT-26), the seqs only in
  the second tape (added), and the seqs only in the first
  (removed), each list in ascending order. Test,
  tests/test_compare.py.
- FT-34. The report is a single JSON object printed to
  stdout, deterministic: comparing the same two files twice
  produces byte-identical output, and comparing a tape with
  itself reports no changes, no added or removed events,
  equal outcomes, and zero deltas. Test,
  tests/test_compare.py.

Amendments recorded in the Round 7 repair. FT-31, a side
that loads and validates but cannot be digested is refused
with a named error, since the report carries digests.
FT-32, an outcome that carries no basis reports its basis as
null, and redacted_values is reported as each side's value
as it stands, whatever its type, because the validator does
not constrain that field; the delta still requires numbers
on both sides. Totals and skipped_lines may be null in a
valid tape and then carry no measures.
