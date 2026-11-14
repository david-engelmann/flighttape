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
  keys sorted by their UTF-16 code units, arrays in order,
  no whitespace anywhere, strings as JSON strings with
  non-ASCII characters written unescaped, the whole encoded
  as UTF-8.
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
valid tape and then carry no measures. A delta whose
difference is not a finite number is omitted, since the
report is JSON and has no standard value for it.

- FT-35. Rollup takes a directory and considers every entry
  whose name ends in .json, in filename order, without
  descending into subdirectories. A path that is not a
  directory is refused with a named error and no report. An
  entry that is not a file is excluded as not a file, a file
  that fails to load is excluded as unreadable, a tape that
  fails validation is excluded as invalid with its defects,
  and a valid tape that cannot be digested is excluded as
  undigestible. Every exclusion is named on stderr in the
  same register as the other commands and recorded in the
  report, and the command exits 1 when any file is excluded
  and 0 when none is, the way convert-dir reports a batch.
  The report is printed whenever the directory exists. Test,
  tests/test_rollup.py.
- FT-36. The report lists each valid tape with its file
  name, digest, outcome status and basis (null when a tape
  carries no basis), duration under FT-32's rule, event
  count, and run totals as recorded, a null totals mapping
  listing as empty. Aggregates cover the valid tapes only:
  the counts of tapes, excluded files, and events; outcome
  counts by status; each run totals key summed over the
  tapes that carry it; redacted_values as the sum of its
  numeric values plus a count of the tapes carrying a
  non-numeric value, the field being unconstrained (FT-32);
  each skipped_lines reason summed; and the duration total
  with the counts of tapes whose duration is known and
  unknown. Test, tests/test_rollup.py.
- FT-37. Two files with identical content are two tapes.
  Both are listed and both count in every aggregate; a
  rollup never deduplicates by digest. The report is
  deterministic: the same directory contents produce
  byte-identical output regardless of the order the files
  were created in or their timestamps. Test,
  tests/test_rollup.py.
- FT-38. The report is a single JSON object printed to
  stdout. A directory holding no .json entries produces the
  report with zero tapes and no exclusions and exits 0.
  Test, tests/test_rollup.py.

Amendments recorded in the Round 8 repair. FT-36, a sum
that cannot be rendered in JSON reports as null: the
redacted_values total when its numeric sum overflows or
runs past finite, and any totals or skipped_lines sum
whose integer passes the interpreter's digit limit for
rendering numbers, a limit a single loaded value cannot
cross but a sum across tapes can. FT-35, an unreadable
exclusion records the loader's reason as its error detail,
as an undigestible exclusion does.

## Round 9. Tape slicing (FT-39 to FT-43)

- FT-39. Slice takes a tape file and a selection, a seq
  range given by --from and --to, both inclusive, either
  bound omittable and defaulting to that end of the tape,
  and an optional --kind filter that may repeat. Bounds
  outside the tape's range are not errors; they select
  what exists. A bound below zero, a --from above its
  --to, or a --kind outside the tape format's kind enum
  is a named refusal, exit 1, nothing written. A selection
  matching no events is likewise a named refusal, because a
  tape must hold at least one event. The source
  is loaded and validated the way the other commands load
  and validate; a source that fails either is refused
  with the failure named, exit 1, nothing written. Test,
  tests/test_slice.py.
- FT-40. The output is a tape carrying the source's
  tape_version and its run identity as recorded (harness,
  session_id, source_format, started_at, skipped_lines,
  redacted_values). Its events are the selected events in
  their original order, each event unchanged except its
  seq, renumbered from 0 in selection order, because the
  validator requires consecutive seq values. Test,
  tests/test_slice.py.
- FT-41. The output records its provenance at
  run.slice_of, the source tape's digest under canonical
  form 1, the range as requested with an omitted bound
  as null, the kind filter as a list or null, and the
  count of events selected. A slice of a slice records
  the immediate source's digest, so a provenance chain
  is walked one digest at a time. Test,
  tests/test_slice.py.
- FT-42. The output's outcome is recomputed over the
  selected events by the same determination the converter
  uses, never carried from the source. Its run totals are
  recomputed over the selected events for the counts the
  events themselves carry, messages and tool_calls, and
  the token totals are omitted, never carried over and
  never estimated, because events do not record usage.
  Test, tests/test_slice.py.
- FT-43. The output tape is validated before anything is
  written; a slice that fails validation is a named
  error, exit 1, nothing written. Output goes to the
  file named by -o or to stdout, and a write failure is
  a named error with exit 1, in the family's register.
  Test, tests/test_slice.py.

Amendments recorded in the Round 9 build. FT-40 as first
written let an empty selection produce a tape with an
empty events array; the validator requires at least one
event, so FT-39 as amended makes an empty selection a
named refusal instead. The outcome basis no-events still
exists for a tape whose events are all sidechain or
session_end events, which a slice can produce.

## Screening, Round 10 obligations, written before the code

- FT-44. The screen command takes one tape file. The tape
  is loaded and validated first; an invalid tape is a
  named refusal in the validate path's own terms, exit 1,
  and no report is produced. Test, tests/test_screen.py.
- FT-45. Screen examines exactly the strings FT-22 names
  as text: message text, tool result content, and every
  string leaf and every string key of a tool call's input
  at any depth. Identifiers, timestamps, the run's
  bookkeeping, and the outcome are outside the rule,
  exactly as they are outside FT-22. Detection runs the
  converter's own redactor: a string the redactor would
  change is a hit, and paths are counted in a string
  exactly as FT-24 counts replacements. One definition
  of an absolute path, FT-23, serves both directions.
  Test, tests/test_screen.py.
- FT-46. The report is a JSON object carrying the tape's
  digest under canonical form 1, the number of strings
  screened, the number of paths found, a hits list, and
  a clean flag. Each hit names the event's seq, the
  field (text, content, or input), whether the paths
  sat in a key or in a value, and how many paths that
  string holds. The report never contains a screened
  string or any part of a path it found; a hit is a
  location and a count, nothing more. Screen exits 0
  when the tape is clean and 1 when any path is found,
  and the report is printed in both cases. Test,
  tests/test_screen.py.
- FT-47. Screen never modifies the tape. Its report goes
  to stdout or to the path named by -o, and an output
  path that resolves to the tape itself is refused by
  name. A write failure is a named error with exit 1.
  Test, tests/test_screen.py.
- FT-48. The directions agree: a tape the converter
  produces screens clean, including the converted
  synthetic fixture, whose paths the redactor already
  replaced; and a hand-built tape carrying paths in
  each of the FT-22 carriers (message text, tool result
  content, an input value, an input key) reports a hit
  in each carrier. Test, tests/test_screen.py.

## Round 10 post-review amendments, recorded before the repairs

- FT-45a. A text or content carrier holding a container
  is screened at any depth, the way the converter's
  routing redacts it under FT-22: every string leaf and
  every string key inside the container is a screened
  string, and its hits carry the carrier's field name.
  A screen that examined only top-level strings would
  pass tapes whose paths ride inside containers, the
  exact leak the converter's routing exists to close.
  The first implementation gated the carriers on being
  strings; the red team drove a path through a
  block-shaped message text that validated and screened
  clean. Test, tests/test_screen.py.
- FT-45b. The screening walk is iterative. No nesting
  depth a valid tape can hold makes screen fail with a
  traceback (FT-21); the first implementation recursed
  and a valid tape nested about 990 deep crashed it.
  A depth the canonical form itself cannot render is
  refused through the digest's named error instead.
  Test, tests/test_screen.py.
- FT-47a. The overwrite refusal also fires when the
  output path names the same file as the tape by
  identity (a hardlink), not only when the resolved
  paths are equal.
- FT-48a. The test file also pins what the first tests
  left loose: the CLI's clean arm (exit 0 with the
  report printed), the tape's bytes unchanged after
  screening with and without -o, a write failure as a
  named error, and the hardlink refusal.

## Round 11. The Rust port (FT-49 to FT-56)

David's ruling of 2026-11-02: the repos are Rust
libraries. This round ports flighttape, and the Python
of Rounds 1 to 10 stands as the behavioral spec the
port must reproduce exactly.

- FT-49. The port reproduces the behavior of Rounds 1
  to 10 exactly: the same ten commands with the same
  outputs, exit codes, defect and error texts, and the
  same digests. Canonical form 1 (FT-26 to FT-28) does
  not change; the golden fixture digest stands.
  Anything the port cannot reproduce is named in the
  round's dossier as a deviation, never silently
  dropped. Test, the ported suite throughout.
- FT-50. The crate is a library (lib.rs, one module
  per Python module) plus a thin binary named
  flighttape carrying the CLI. The only dependency is
  sha2, for the digest; everything else is the Rust
  standard library, matching the Python's stdlib-only
  discipline. The repository's CI gate becomes cargo
  test plus the codepoint scan. Test, Cargo.toml and
  the gate run.
- FT-51. The Rust JSON reader accepts what Python's
  json accepts for a tape file: the NaN and Infinity
  literals as non-finite numbers (named downstream,
  never smuggled), duplicate keys with the last value
  standing, integer literals of up to 4300 digits held
  exactly, and nesting counted whole at any depth the
  file carries. It refuses the rest with the loader's
  standing phrases. A hand parser carries this; no
  general JSON crate's acceptance rules substitute
  for the spec's. Test, tests in the reader module
  and the ported robustness suite.
- FT-52. Numbers render under FT-26 unchanged. The
  Rust renderer computes the value's exact decimal
  expansion by integer arithmetic and takes the
  shortest prefix that round-trips, rounding a cut
  that lands exactly on the half to the even digit,
  as the ECMAScript algorithm and the Python both
  do; the standard library's own float formatting
  rounds those ties away from even and cannot be
  used (found by the swept corpus before the code
  landed: the double 2163015292276093.25 renders
  ...3 there, ...2 in the spec). The ported number
  table and the swept corpus pin the agreement
  digit for digit. Integer rendering refuses a
  digit string past the 4300-digit limit with the
  FT-28 phrase, as the Python does. Test, the
  identity module's tests.
- FT-53. Text is held as UTF-16 code units where the
  spec's own rules are stated in them: keys sort by
  UTF-16 code units, and a string carrying an unpaired
  surrogate is refused at canonicalization with the
  FT-28 phrase, as the Python refuses it. Everywhere
  else a string with an unpaired surrogate is ordinary
  non-matching text, as it is to the Python's
  comparisons. Test, the identity and validate
  modules' tests.
- FT-54. The canonical renderer refuses a tape nested
  past 1000 containers with the FT-28 phrase. This
  fixes a deterministic number where the Python's
  refusal rides the interpreter's recursion limit;
  the value is chosen at the Python's own order of
  magnitude, and the difference is recorded here
  rather than discovered by a consumer. The
  validator's iterative walks are uncapped, as the
  Python's are. Test, the identity module's tests.
- FT-55. The Python test suite's cases port to cargo
  tests under the same intent, file for file where
  the module maps, including the golden digest, the
  playback golden, and the number table. Cases that
  exist only because Python values can be cyclic or
  wrongly keyed in memory have no Rust counterpart,
  since the Rust value type cannot be built in those
  shapes; each such case is named in the dossier.
  The Python files leave the tree only in the
  landing state of this round, when the port has
  passed every gate the Python passed. Test, cargo
  test on the final head.
- FT-56. Before the port lands, a differential
  harness runs both implementations over the
  synthetic corpus (the repository fixtures and
  generated tapes and session files, all synthetic)
  and byte-compares stdout, stderr, exit codes, and
  written files for every command. A single
  differing byte is a port defect to fix, not a
  difference to document. The harness and its corpus
  live with the round's records; the ported cargo
  suite is what ships. Test, the round's dossier.

## Round 12. Rendering edges and the timestamp stray digit (FT-57 to FT-62)

The dispositions collected after Round 11, each
decided from the surfaces the code actually feeds.
The crate's rendering helpers split by surface:
render_number feeds canonical form 1, which FT-26
defines as the ECMAScript rendering; the pyrepr
helpers feed only CPython surfaces, the validator's
defect phrases, the dumps documents, and the str()
renderings in conversion and playback. A helper is
held to the standard of the surface it feeds.

- FT-57. render_number's shortest search takes the
  shortest digit sequence that round-trips, trying
  both neighbours of the exact expansion at each
  width. The Round 11 search rounded the exact
  expansion at the cut, half to even, and accepted
  the first width whose rounded candidate parsed
  back; at an exact power of two the rounding
  interval is asymmetric, the gap below half the
  gap above, so the tie-to-even candidate below
  can fall outside the interval while the
  candidate above sits inside it, and the search
  walked past the shortest width. The recorded
  instance: 2^-24, whose exact expansion
  59604644775390625 terminates one digit past the
  shortest sequence 5960464477539063; the crate
  emits the 17 digits, the ECMAScript algorithm
  the 16. This is a defect against FT-26 itself,
  not a CPython edge: the helper is not
  ECMAScript-faithful for the class, so it is
  fixed, never documented as faithful. FT-52's
  wording is clarified by this item: the shortest
  prefix phrase meant the shortest sequence that
  round-trips, and the search now considers the
  candidate above the cut whenever the rounded
  candidate does not round-trip, before moving to
  the next width. The class is exact powers of
  two whose expansion terminates just past the
  shortest width on an exact tie; the crate's own
  corpora did not exercise it (the number sweep's
  only fraction-zero entries are 1, -1, and 0.5,
  and the golden fixture tape carries no float
  values, its dotted numerals being timestamp
  text), so the golden digest stands unchanged.
  The pin: the number sweep gains the class,
  every representable power of two from 2^-1074
  to 2^1023, 2^-24 named among them, its expected
  renderings generated from CPython repr's digit
  sequence presented the ECMAScript way, a path
  independent of the crate's exact-expansion
  search (the two choose the same shortest
  sequence for every double). What the pre-port
  Python renderer did on the class is recovered
  from the tree's history and reported in the
  dossier either way. Test, tests/identity.rs.
- FT-58. repr_float is CPython-exact. Its surfaces
  are all CPython surfaces: the dumps documents
  the commands print, the defect phrases
  repr_value builds for the validator, and the
  str() renderings conversion and playback make
  through py_str. Round 11 recovered the digits
  from render_number's presented string, and a
  presentation is not a digit sequence: the
  ECMAScript integer presentation of
  1000000000500000000 appends zeros past the
  eleven shortest digits, and the recovered
  digits carried them into the scientific band,
  rendering 1.000000000500000000e+18 where
  CPython renders 1.0000000005e+18. The digits
  now come from the shortest-digit computation
  itself, the same computation FT-57 corrects,
  and presentation zeros never enter them. The
  pin is a differential sweep against CPython
  repr, not a sample: a repr corpus generated
  from the interpreter over the number sweep's
  bit patterns, the band edges at exponents -5
  to 17, and the padding shapes, integral values
  whose shortest digits are short and whose
  presentations are long, asserted through
  repr_float directly and through the dumps and
  repr_value paths. Test, tests in the pyrepr
  module and the ported suites it feeds.
- FT-59. repr_text's printability is CPython-exact.
  repr_text feeds only repr surfaces, the
  validator's defect phrases and the container
  arm of py_str; no ECMAScript surface consumes
  it. The Round 11 table was a hand approximation
  (its own comment said so) and it disagrees with
  CPython at U+0378, U+0379, U+1AFF, and U+E0001,
  unassigned codepoints sitting in the table's
  gaps, which CPython treats as non-printable and
  escapes. The approximation is replaced by a
  table generated from CPython, the generating
  interpreter and its Unicode version recorded
  in the test as the fold pins record theirs,
  and the pin sweeps the whole codepoint space:
  for every scalar value the crate's printability
  agrees with the generated table, the four
  recorded points named in the test. A four-point
  patch is not the fix; the sweep is what proves
  no further gap stands. Unpaired surrogates stay
  escaped by construction, as before. Test, tests
  in the pyrepr module.
- FT-60. The timestamp grammar refuses a stray
  digit at the end of the string. CPython's
  fromisoformat consumes one leftover digit after
  the time's last component only when the string
  continues; the crate's time_part consumed it
  unconditionally, so 2026-01-01T00:00:005,
  2026-01-01T00000, 2026-01-01T01:010, and
  2026-01-01T0000005 parse here and refuse in the
  interpreter. The fix is exactly this: a
  one-digit run after the last component is
  consumed only when a further character follows
  it; when the run reaches the end of the string
  the parse refuses. Nothing else in the grammar
  moves; the continuation forms the corpus
  already pins (the digit before Z at
  2026-01-01T09:00:005Z and 2026-01-01T09005Z,
  the two-digit fraction at
  2026-01-01T0900055Z) stay accepted.
  tests/fixtures/timestamp_corpus.txt gains the
  four shapes above as REFUSE lines. The consumer
  consequence is stated so no reader mistakes
  the scope: onrecord and wallofwrong pin the
  crate and named this acceptance as a deviation
  in their Round 3 dossiers; they re-pin when
  this lands, and their deviations retire in
  their next rounds, not in this one. Test,
  tests/timestamps.rs over the corpus.
- FT-60a. One separator character may stand
  between the time and the offset, and a component
  colon may stand before the offset's sign. The
  grammar's earlier pinning never probed a
  separator in that position; the interpreter
  allows exactly one. The rule, probed against
  fromisoformat at every boundary: after the
  time's last component, in the extended and the
  basic form alike, one separator character may
  precede the offset's sign (the Z form reaches
  the position as +00:00 after the replacement,
  so a separator before Z is this same case). The
  separator is any single ASCII character, a
  space, a tab, NUL, a letter, the colon itself;
  a multi-byte character is not skipped (U+2003
  and U+00E9 refuse), two separator characters
  refuse, and the sign must follow the separator
  immediately, so a separator at the end of the
  string or before anything but the sign refuses.
  The skip is offered only where nothing else
  intervened: after a fraction (behind its point,
  the basic form's leftover digit run, a fraction
  carried on the hour) or after the stray digit
  FT-60 consumes, a separator refuses. And in the
  extended form, a colon introducing the minute
  or the second and standing immediately before
  the offset's sign leaves that component zero
  and the offset follows (2026-01-01T00:+05:30
  reads its minute as zero with the offset
  intact; 2026-01-01T00:00:+00 its second); a
  colon followed by anything but the sign or the
  component's two digits refuses as before, a
  colon at the end of the string, a colon and a
  space, a colon and one digit alike.
  tests/fixtures/timestamp_corpus.txt gains the
  lines pinning each boundary, the accepts with
  their instants and the refuses. Test,
  tests/timestamps.rs over the corpus.
- FT-61. The CodeRabbit disposition closes on
  evidence, with no code obligation. The advisory
  pass on PR 11 never posted findings: the PR
  holds no review comments and no reviews from
  the bot, and its two bot issue comments are a
  failure notice (comment 6597505419, "Review
  failed, the pull request is closed", a
  walkthrough with no findings list) and an
  action-not-completed notice (comment
  6597872800), both consequences of the merge
  landing while the pass was still processing.
  There is nothing to fix and nothing to dismiss;
  the dossier records the absence as read from
  the comment lists, and the item is discharged.
- FT-62. The round's verification shape. The
  FT-56 differential harness re-runs over its
  standing corpus, and its outcome matches the
  previous round's recorded result: every
  divergence it reports is one already recorded
  there, proven pre-existing on the unmodified
  tree or repaired in this round, and no new
  divergence stands; the new
  pins of FT-57 to FT-60 run as cargo tests; the
  golden digest test stands unchanged. Canonical
  bytes change only for a tape carrying a float
  in FT-57's class, and the dossier enumerates
  any standing-corpus tape whose digest moved,
  with none expected: the golden fixture tape
  carries no member of the class, and the harness
  surfaces it if that expectation is wrong. The
  full local gate (cargo test, clippy with
  warnings denied, fmt, the codepoint scan)
  passes on the final head before the round's
  reviews, as every round.
- FT-63. Round 13 adopts the fraction law the
  probe grid measured (277 probes, report at
  chat2-ft-round13/probe-report.md). The colon
  is a fraction separator after the seconds,
  in the time and in the offset: the separator
  candidate step after each component consumes
  exactly one character, and a colon standing
  after the seconds is eaten by that step, so
  what follows it is fraction digits. No colon
  group, concatenation, or nesting exists. The
  FT-60 stray digit, the FT-60a skipped byte,
  and the empty fraction are faces of this one
  step.
- FT-64. The fraction window: with R characters
  remaining in the time part after the fraction
  separator, exactly min(R, 6) characters are
  read and all must be digits; the value is
  those digits right-padded to six. Digits past
  the window are skipped. This holds for ".",
  ",", and ":" separators and for basic times,
  whose fraction is bare digits after the
  seconds under the same rules.
- FT-65. The unexamined leftover: when R is
  greater than 6, once the window digits and
  the skipped digits end, any remaining
  characters of the time part before the
  offset boundary are not examined at all, and
  the string accepts with the same instant, as
  long as an offset boundary follows; a string
  ending inside the leftover refuses. The
  offset boundary is the first "Z", "+", or
  "-" byte of the time string. The leftover
  rule is time only: the offset substring must
  be consumed exactly, so a non-digit leftover
  after an offset fraction refuses, while
  skipped digits remain free there.
- FT-66. Offset values under colon fractions
  follow the standing rules unchanged: the
  drop rule (a zero whole-seconds part drops
  the fraction entirely) and the strict
  under-24-hours bound applied to the enlarged
  total.
- FT-67. The round extends the timestamp
  corpus with the grid's candidates (accepting
  and refusing shapes mixed), and verifies by
  the full local gate plus a differential run
  of the grid generators against the crate.
  The Round 12 owed item on late CodeRabbit
  findings is discharged here: the findings
  arrived during Round 12 and were fixed
  in-round, so nothing is owed from it.
- FT-68. Amendment, from the Round 13 red team:
  the interpreter reads a lone NUL byte as the
  end of the string at its completion points
  (measured: after the hour, minute, and
  seconds in both time modes, after a full
  six-digit fraction, after a complete offset,
  and in the date and time separator position),
  while a NUL anywhere else refuses (inside a
  component, after a short fraction, after the
  date alone, doubled, or followed by more
  content). The crate models the lone NUL as a
  terminator at exactly the measured points.
  The repair's probe pass fixes the boundary
  cells the first battery left open (including
  NUL after skipped fraction digits and after
  each offset component) before the rule is
  implemented, and the corpus gains the NUL
  cells. The corpus count guard also becomes
  an exact pin in this repair, per FT-58's
  precedent.
- FT-69. Round 14 is corpus hygiene, and it
  changes no behavior. Two lines in the
  timestamp corpus stand duplicated (the offset
  drop case and the colon-group refusal, each
  appended twice across Rounds 12 and 13); the
  duplicates are removed, the corpus stands at
  371 probes, and the corpus count guard in
  tests/timestamps.rs moves from 373 to the
  exact 371 in the same step, per the FT-58
  precedent that a generated set's guard pins
  its exact count. The number sweep corpus
  stands at 6,284 entries, all unique; its
  guard in tests/identity.rs, an inequality
  today, becomes the exact pin in the same
  round. No other corpus, guard, or source file
  moves.
- FT-70. The CodeRabbit disposition owed from
  Round 13 discharges here on evidence: the
  Round 13 pull request carries zero inline
  findings on read-back (the bot's later reply
  was "Action not completed - Pull request is
  closed," checked 2026-11-04), so there is
  nothing to fix and the absence is the
  disposition.
- FT-71. README records 14 of 100 only in the
  closure commit written after this round's
  three reviews and owner verification.
- FT-72. The CodeRabbit disposition owed from
  Round 14 discharges here on evidence read
  back on 2026-11-06. PR 14 carries zero review
  comments and zero reviews from the bot. Its
  four issue comments are the owner's trigger
  (comment 6652814021), the bot shell (comment
  6652816969, "Review failed, the pull request
  is closed," a walkthrough of the corpus guard
  changes with no findings list and merge risk
  Low), the owner's landing note (comment
  6652864479), and the bot's action notice
  (comment 6652930573, "Action not completed,
  pull request is closed"). The shape is the
  failed-because-closed shape of FT-61. No
  finding was ever posted, so there is nothing
  to fix and nothing to dismiss, and the item
  is discharged.
- FT-73. Round 15 takes up the one atomic
  write in the crate, the screen report's
  sibling temp file (FT-47). Every other write
  is a plain direct write on both sides, the
  converter's, convert-dir's, and slice's
  outputs matching the oracle's plain write
  calls, and none of them carries a temp seam.
  The oracle's screen write is tempfile.mkstemp
  with the output's parent as dir and the
  output's name plus a dot as prefix, then
  os.replace over the spelled output. mkstemp's
  candidate carries a suffix of exactly eight
  characters, is created exclusively with mode
  0600, and its dir is absolutized lexically,
  the current directory joined and the result
  normalized with no link resolution. The port
  at the round's branch point instead joined
  the spelled parent to the output name plus a
  dot plus the decimal process id, and wrote it
  with a single create-or-truncate write before
  the rename. Twin probes against the oracle
  measured the three arms of that difference,
  and FT-74 to FT-77 state each one and its
  repair.
- FT-74. The width law. The candidate's
  effective overhead past the output name is
  one dot plus exactly eight characters, so a
  plain output name of 246 bytes writes and
  247 bytes answers the cannot-write refusal,
  at every process id. The branch-point port's
  overhead was one dot plus the pid's decimal
  digits, measured at 6 under the ambient
  five-digit pids (writes through 249, refuses
  at 250) and at 2 in a pid namespace where the
  binary runs as pid 1 (writes through 253,
  refuses at 254), against the oracle's 246
  and 247 in both. The repair draws a fresh
  suffix of exactly eight characters for each
  candidate. The boundary is pinned at the CLI
  in both pid postures, plain and through a
  popped parent.
- FT-75. The placement law. The temp file is
  created under the lexically absolutized
  parent of the output, while the rename target
  stays the spelled output path. Where a popped
  symlink makes the lexical parent exist and
  the physical parent not, creation succeeds at
  the lexical parent and the rename refuses
  with the cannot-write message, leaving no
  file. Where the physical parent exists and
  the lexical parent does not, the oracle
  refuses with the cannot-write message and
  writes nothing, and the branch-point port
  wrote the report into the physical directory
  instead. The repair creates the temp under
  the lexical parent, and both directions are
  pinned, the physical direction at a short
  name and at 246 bytes.
- FT-76. The candidate is not predictable.
  mkstemp creates its candidate exclusively and
  draws again on a collision, so an existing
  file or directory at any one candidate name
  is never truncated, consumed, or a refusal.
  The branch-point port's candidate was the
  process id itself, and the probes measured a
  planted file at that name consumed by the
  write and rename, and a planted directory at
  that name refusing the report outright, where
  the oracle wrote the report and left both
  standing. The repair creates each candidate
  with create-new semantics and draws again on
  AlreadyExists, up to 64 attempts, creates the
  temp with mode 0600 as mkstemp does, removes
  the temp on any failure after creation, and
  keeps the rename target and the cannot-write
  message as they stand. The planted shapes at
  the old deterministic name are pinned.
- FT-77. The byte-counting arm. The temp
  candidate is assembled from the output file
  name's raw OS bytes end to end, never through
  a lossy text conversion, so the FT-74
  boundary counts bytes, one raw byte counting
  once. The branch-point seam read the name
  through to_string_lossy, the SB-37 shape. At
  the CLI the raw-byte cells measure a separate
  and older posture first, the entry point's
  std::env::args collection panics on a
  non-UTF8 argument and the library wrapper
  returns 1, where the oracle writes its short
  and 246-byte raw names and refuses the
  247-byte one. That posture belongs to the
  whole argv layer and predates the seam, and
  this round does not widen into it. The seam's
  own byte law is repaired here and pinned by
  unit tests that hand the write raw-byte
  paths directly, at 246 and 247 bytes.
- FT-78. The round's verification shape. The
  twin probe battery re-runs against the
  repaired binary, and every cell agrees except
  the raw-byte CLI cells whose divergence
  FT-77 names to the argv layer. A mutant with
  the suffix widened past eight fails the
  boundary cells and no others, and a mutant
  with the candidate name built lossily fails
  the FT-77 unit pin. The FT-56 differential
  harness re-runs in its standing shape with no
  new divergence, the corpus guards stand exact
  at 371 and 6,284, and the full local gate
  (cargo test, clippy with warnings denied,
  fmt, the codepoint scan) passes on the final
  head before the round's reviews, as every
  round.
- FT-79. README records 15 of 100 only in the
  closure commit written after this round's
  three reviews and owner verification.
- FT-80. Two wording corrections to this
  round's own clauses, on the reviews'
  Minors. FT-78's mutant sentence is scoped
  to the battery that ran: the width mutant
  was run over the 32 boundary cells only
  (16 lengths in each pid posture), failing
  14 of them, all at lengths 240 to 246 and
  none at 247 or beyond; no absence outside
  the boundary lengths was measured, and
  none is claimed. FT-74's pins are
  enumerated by instrument: the cargo tests
  pin the boundary at the ambient pid, and
  the twin probe battery pins it at pid 1
  under a pid namespace and through both
  popped-parent directions at the ambient
  pid; the popped form at pid 1 was not run.
  The red team adds one arm the lane's
  snapshot did not compare: the shipped
  seam landed reports at mode 0644 under
  umask 022 where the Python lands 0600,
  and FT-76's mode 0600 covers it; the
  repair closes it.

- FT-81. The CodeRabbit disposition owed
  from Round 15 discharges here on evidence
  read back on 2026-11-07. PR 15 carries
  exactly two review comments, both the
  bot's and both before the merge: comment
  4492864381 on src/cli.rs, the root
  output's temporary parent, posted
  2026-11-06T13:23:47Z, and comment
  4492864412 on tests/screen.rs, the
  boundary test's filesystem assumption,
  posted 13:23:48Z. Its one review
  (5800660627, COMMENTED, 13:23:49Z) states
  two actionable comments posted. Its four
  issue comments are the owner's trigger
  (6685759206), the bot shell (6685762788, a
  completed walkthrough holding no Review
  failed marker and no findings beyond the
  two inline comments, last updated
  13:23:54Z), the bot's second action
  notice (6685907255, "Action not
  completed, Already reviewed the last
  commit"), and the owner's landing note
  (6685931904). The merge landed at
  13:27:08Z; every bot timestamp precedes
  it and nothing arrived after, so no late
  finding exists, and this read-back
  confirms the merge-time record whole.
  Both findings stand dismissed for the
  reasons recorded when they were answered:
  the root output's failure is total on
  both sides, a twin probe of -o / showing
  both exit 1 with byte-identical stderr
  and no residue, and the 255-byte
  component limit is this contract's
  measured law on the target filesystem,
  verified on this box and in CI, so sizing
  the boundary test down to a probed limit
  would unpin the law the round existed to
  pin.
- FT-82. Round 16 brings the README to the
  fleet storefront standard. The first
  screen stands in order: the title, a lede
  identical to the repository description,
  one short paragraph, the one ci badge,
  and one block of captured output. Then
  Install, Quickstart, Usage, Status, and
  License. The badge is verified: the ci
  workflow (403262171) stands active, its
  latest run on main at this round's base
  completed success on 2026-11-07, and the
  badge image renders "ci - passing". The
  Install block runs verbatim from a clean
  state into a scratch CARGO_HOME, and
  every captured block is output from runs
  over a synthetic session written for the
  page; no real trace stands anywhere in
  it. The copy bar's one exception is
  recorded here: the repository description
  carries a colon, and it stands verbatim
  in the lede and nowhere else in the
  README's prose.
- FT-83. The storefront checklist's item 3
  does not pass and is recorded open. The
  repository description reads "Every agent
  run, from every harness, lands as one
  flight tape: events, tool calls, diffs,
  spend, timings, and the outcome, in a
  bundle any tool can read back." The
  README's lede matches it byte for byte
  from this round. Cargo.toml's description
  is a third variant, "Flight tapes for
  agent runs: validate, convert, play back,
  compare, and screen recorded sessions.",
  named here and not repaired inside this
  round's README-only bound.
- FT-84. The round's verification shape.
  The full local gate passes on the final
  head (cargo test at the standing 247,
  clippy with warnings denied, fmt, the
  codepoint scan over every tracked file),
  and the FT-56 differential harness
  re-runs in its standing shape with no new
  divergence: 117 cases, the one recorded
  divergence at case 64 standing in the
  FT-54 band, zero file divergences, and
  the corpus guards exact at 371 and
  6,284.
- FT-85. README records 16 of 100 only in
  the closure commit written after this
  round's three reviews and owner
  verification.
- FT-86. The three reviews of this round
  found one shared defect in the README,
  and this item records it and its
  repair. The Usage sequence as first
  printed implied that the rollup block
  reads the directory the convert-dir
  block builds, and following the page
  in one directory could not produce
  the output shown. convert-dir names
  its tapes after their source sessions
  and writes a summary.json beside
  them, so the directory it builds
  holds session-named tapes and a
  summary, and a rollup over that
  directory, measured firsthand in the
  closure, lists the summary as
  excluded with its defects named on
  stderr and exits 1. The rollup the
  page printed ran instead over a
  separately staged directory holding
  the two individually converted tapes.
  Every block was a genuine capture,
  and the capture ledger stated the
  staging plainly, but the page did
  not. The repair restages the chain.
  The two tapes now come from the
  convert steps the page shows, a
  mkdir and two converts writing
  tapes/run-a.tape.json and
  tapes/run-b.tape.json, every later
  Usage command reads the tapes at
  those paths, and the rollup block
  runs over that directory with its
  captured output unchanged.
  convert-dir stands on its own,
  writing to a directory of its own,
  and its lead-in states its real
  behavior, the session-named tapes,
  the summary.json beside them, and
  the rollup over such a directory
  counting the summary as excluded
  and exiting 1. The repaired sequence
  was run in one clean directory in
  the page's order against the tip
  binary, and every shown output
  compared byte for byte with the page
  and with the finisher's banked
  captures, eighteen checks of
  eighteen passing. Two substitutions
  stand recorded rather than hidden.
  The convert steps' file writes and
  the slice step's are refused by the
  sandbox write fault this session, so
  their bytes were staged from the
  banked tapes and checked through
  the binary's stdout forms,
  hash-paired at every step. And
  convert-dir's live re-run was
  refused across the session's
  attempts and rides a spaced driver;
  its shown line stands on the
  finisher's hash-paired capture, and
  its output directory's names and
  bytes were re-verified on the
  finisher's genuine staging.
- FT-87. CodeRabbit's review of this
  round's pull request (PR 17) posted
  four findings, all of them on the
  README, and this item disposes of
  them. Two are repaired here. The
  first screen now states the
  converter's current scope in its
  introduction, that the converter
  reads Claude Code session files,
  where before that fact stood only
  in Status. And the two synthetic
  session files the Usage examples
  run on are now shipped in the
  repository under examples/, and the
  Usage preamble stages the working
  directory from them, so the whole
  sequence reproduces from a clean
  checkout. The shipped files are
  byte-identical to the capture
  inputs, hash-paired in the lane
  record. Two findings are dismissed
  with their reasons. The Install
  block needs no added PATH prose,
  because the block itself puts the
  scratch installation's bin
  directory on PATH and was verified
  verbatim in that form. And the
  Quickstart's outputs stand without
  shell comment marks, because the
  block is a captured transcript in
  the fleet's uniform form, not a
  script. This item lands after the
  closure commit by the server-side
  sequence this round used, and the
  rounds line the closure set is
  untouched and still exact.
- FT-88. The CodeRabbit disposition owed
  from Round 16 discharges here on evidence
  read back on 2026-11-09. PR 17 carries
  nine reviews, twelve review comments,
  and four issue comments. Its one
  substantive review (5815938269) states
  four actionable comments posted, and the
  four are the review comments on the
  README: 4504494477 (the introduction's
  scope), 4504494497 (the Usage examples'
  shipped inputs), 4504494486 (PATH prose
  at the Install block), and 4504494494
  (comment marks in the Quickstart block),
  all Minor. FT-87 disposed of the four
  before the merge, two repaired and two
  dismissed, each with its reply on its
  own thread, and three of the bot's
  acknowledgments accept the answers, one
  withdrawing its suggestion outright.
  The fourth acknowledgment (4504512381,
  posted 2026-11-08T01:33:46Z, after the
  merge at 01:33:21Z) does not accept. It
  states that the Install block in the
  README sets neither CARGO_HOME nor PATH,
  which differs from FT-87's description,
  and asks whether another revision should
  be checked. Read back firsthand in this
  round, the bot is right on the fact. The
  block at the merged head f16d9b3b and on
  main is the three commands, clone, cd,
  and cargo install, and it sets neither.
  The scratch CARGO_HOME and the PATH
  entry were the verification arrangement
  FT-82 records around the block, not
  lines in it, and FT-87's sentence, the
  thread reply (4504510615), and the
  round's landing note each misdescribe
  the block on that point. This item
  corrects the record. The dismissal
  itself stands on the corrected ground:
  the block runs verbatim as printed for
  a reader holding the stated
  prerequisites, Git and a Rust toolchain,
  and a toolchain install places its own
  bin directory on the PATH where cargo
  install lands the binary, so no added
  PATH prose is owed. There is no other
  revision to check, f16d9b3b is the
  merged head, and a reply on the thread
  records this item as the answer. The
  pull request's bot shell is a completed
  walkthrough updated through f16d9b3b
  with merge risk Low and no findings
  beyond the four, its second notice
  (6715616222) is the action-not-completed
  form stating the last commit was already
  reviewed and holds no findings, and no
  Docstring Coverage item stands anywhere
  on the pull request.
- FT-89. FT-26's statement now carries
  its Round 6 amendment in its own text.
  The item as first written sorted keys by
  Unicode code point, and the Round 6
  repair recorded in the amendments
  paragraph after FT-30 that keys are
  sorted by their UTF-16 code units, the
  ordering the JSON canonicalization
  scheme uses. That ordering clause is
  folded into FT-26 itself, so the item
  reads whole in one place. The amendments
  paragraph stands unedited as the record
  of when the correction was made, and
  this item is the record of the fold,
  because this contract's convention is
  that a correction is recorded in writing
  and never made silently. The fold
  changes no behavior and no wording
  beyond the ordering clause. The renderer
  already sorted by UTF-16 code units
  (FT-53), the golden fixture digest
  stands at
  cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3,
  and the order is re-pinned at the
  shipped binary by this round's
  measurement under FT-91.
- FT-90. The README names the tape
  format's normative home. A sentence in
  the introduction states that the format
  is specified in CONTRACT.md and in the
  schema at schema/tape.schema.json, so a
  reader implementing the format from this
  repository alone finds the specification
  where the tool is introduced, the
  contract's items and the machine
  readable schema together.
- FT-91. The canonical surfaces are
  measured fresh at the shipped binary,
  against an independent oracle in Python
  that sorts keys by their UTF-16 code
  units and renders floats from the
  CPython repr digit sequence presented
  the ECMAScript way, the independent path
  FT-57 established. The battery runs 54
  cells and all 54 hold. Twelve key cells:
  the divergence pair U+E000 and U+10000,
  whose Unicode code point order and
  UTF-16 order disagree, digests in UTF-16
  order at the binary; a six key mixed set
  across that boundary; astral against
  astral and low BMP against astral
  controls, where the orders agree; a
  shared prefix diverging at its second
  character; a duplicate key standing
  after escape decoding (FT-51); the empty
  key; the order applied at depth in
  nested objects; a surrogate pair spelled
  by escapes digesting as the literal
  character; and the characters either
  side of the surrogate range. Unpaired
  surrogates in a key and in a string
  value each refuse at digest with the
  text that cannot be encoded named, exit
  1 (FT-28 as amended, FT-53). Twenty
  eight float cells: twenty four
  renderings including the point
  thresholds at 21 (1e20 in full, 1e21 as
  1e+21) and at minus 6 (1e-6 in fixed
  form, 1e-7 in exponent form), negative
  zero, the smallest subnormal 5e-324, the
  largest finite double, the FT-52 tie
  instance 2163015292276093.25 rendering to
  the lower neighbour, and 2 to the minus
  24 rendering 5.960464477539063e-8; three
  spelling invariance cells, 1e-7 under
  four spellings digesting once among
  them; and a non-finite value refusing
  with the non-finite number named.
  Fourteen stability cells over one tape:
  the golden fixture converts, validates,
  plays, and digests to
  cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3,
  verifies against that digest in either
  letter case, refuses a wrong digest with
  the mismatch named, compares against
  itself naming the digest on both sides,
  rolls up from its directory with the
  digest carried in the report, slices to
  its first two sequence numbers with the
  source digest recorded in the slice's
  run block and the slice validating, and
  converts again through convert-dir to
  the same digest beside its batch
  summary; and a crafted tape carrying the
  divergence pair as tool input keys, with
  the edge floats as their values,
  validates and digests to the oracle's
  digest of the whole tape. FT-77's
  posture stands measured once more: a
  non-UTF8 argument ends the run at exit 1
  on the argument collection panic and no
  digest is printed. No cell convicts and
  no repair is owed.
- FT-92. This round's verification runs on
  the final tree. Cargo test 247 passed,
  cargo clippy --all-targets -D warnings
  clean, cargo fmt --check clean, and the
  codepoint scan over every tracked file
  at U+2014=0 and U+2013=0. The FT-56
  differential harness runs against the
  Python oracle on this round's binary:
  117 cases, one divergence at case 64 in
  the FT-54 band exactly as Round 16
  recorded it, zero file divergences, and
  the corpus guards stand exact at 371 and
  6,284. The round changes no behavior,
  one contract item's wording, one README
  sentence, and this round's records, so
  the harness's Round 16 result is
  re-measured here rather than assumed,
  and it stands unchanged.
- FT-93. README records 17 of 100 only in
  the closure commit written after this
  round's three reviews and owner
  verification.
- FT-94. The CodeRabbit disposition for
  this round's own pull request discharges
  here, on the read-back of PR 18 taken on
  2026-11-09. The pull request carries one
  review (5829637950, COMMENTED, actionable
  comments posted: 2, with one nitpick in
  its body), two review comments, and a
  completed walkthrough shell; its second
  notice (6746380776) is the
  action-not-completed form stating the
  last commit was already reviewed. No
  Docstring Coverage item stands anywhere
  on the pull request. The two actionable
  items are both Minor and both dismissed.
  The first (4515589769, on this contract)
  asks that the PATH dismissal recorded at
  FT-88 be corrected and the Cargo
  installation directory documented,
  because a reader-chosen CARGO_HOME or
  install root can sit outside the PATH.
  Dismissed: that arrangement is a
  reader's own non-default configuration,
  true of any tool cargo distributes, and
  it does not touch the ground FT-88
  states, that the block runs verbatim as
  printed under the stated prerequisites,
  the default toolchain install placing
  the cargo bin directory on the PATH
  where cargo install lands the binary.
  The second (4515589782, on the README)
  holds that the rounds line records 17
  before the reviews and owner
  verification FT-93 names. Dismissed: the
  closure commit on the round's branch is
  this contract's standing sequence (FT-85
  recorded the same for Round 16), the
  count takes effect only when the pull
  request merges after those reviews, and
  main reads 16 of 100 until it does. The
  nitpick is repaired: it asked for a
  regression pin on keys sharing an astral
  prefix, whose order turns on the code
  units after a shared surrogate pair, and
  the UTF-16 ordering test in
  tests/identity.rs now carries that pair
  in reversed insertion order against the
  complete canonical output. The suite
  stands at 247 after the repair, the pin
  riding inside the existing test. This
  item lands after the closure commit by
  the server-side sequence this round
  used, and the rounds line the closure
  set is untouched and still exact.
- FT-95. The read-back of PR 18, taken
  whole in this round before any other
  work. The pull request's full record is
  five reviews, six review comments, and
  five issue comments, every body read to
  its end. One review is substantive
  (5829637950, COMMENTED, on the
  pre-amendment head cf392c49, actionable
  comments posted: 2, with one nitpick in
  its body); the other four are empty
  COMMENTED carriers. The six review
  comments are the two findings below,
  the owner's two replies, and the bot's
  two follow-ups. The first finding
  (4515589769, Minor, on this contract)
  asked again that the PATH dismissal
  FT-88 corrected be replaced with Cargo
  installation directory guidance. It
  stands dismissed at FT-94 on the ground
  FT-94 states, a reader-chosen CARGO_HOME
  or install root outside the PATH is the
  reader's own non-default configuration,
  true of any tool cargo distributes, and
  the bot's follow-up (4515677055)
  acknowledges the record. The second
  finding (4515589782, Minor, on the
  README) held the rounds line recorded
  17 before the reviews and owner
  verification were complete. It stands
  dismissed at FT-94 on the standing
  sequence, and the bot's follow-up
  (4515683506) withdraws the finding
  itself and resolves the thread; its
  closing note, that the bot had not
  verified the reviews' completion, is a
  caveat and not a finding, and the merge
  recorded at 6747241722 is the completion
  it asked to see verified. The nitpick
  asked for a regression pin on keys
  sharing an astral prefix; it stands
  repaired on the amendment head
  8284aeac, and the pair stands verified
  in tests/identity.rs on this round's
  base, the keys U+10000 followed by b and
  by a inserted in reverse and asserted
  against the complete canonical output.
  The shell (6746176204) stands in its
  final form as a rate-limit notice
  (Review limit reached, run 2f4679c5)
  over the amendment range cf392c49 to
  8284aeac across CONTRACT.md and
  tests/identity.rs, the first review's
  walkthrough retained below the banner,
  Merge Risk Low, and pre-merge checks 4
  passed, Description, Title, Linked
  Issues, and Out of Scope, with no
  further rows; the amendment range was
  never bot-reviewed, and the round's
  three reviews are its review of record.
  The action-not-completed notice
  (6746380776, already reviewed the last
  commit) stands enumerated here and at
  FT-94. Two compressions in FT-94's own
  census are corrected on this record:
  FT-94 says two review comments where
  the record holds six, and a completed
  walkthrough shell where the shell's
  final form is the rate-limit form
  above. No disposition's ground is
  falsified by anything on the pull
  request: FT-88's correction stands
  untouched, the PATH dismissal rests on
  its restated ground and not on FT-87's
  premise, and the rounds dismissal was
  overtaken by the bot's own withdrawal.
- FT-96. The canonical surfaces are
  measured fresh at the binary built from
  this round's base tree, the worktree
  hash-verified blob for blob against the
  remote tree before the build. The
  battery of FT-91 runs again in full: 54
  cells, all 54 hold, the twelve key
  cells, the twenty eight float cells,
  and the fourteen stability cells over
  the golden fixture, which digests to
  cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3
  exactly, and FT-77's posture cell holds
  as contracted. The FT-56 differential
  harness runs against the Python oracle
  on this round's binary: 117 cases, one
  divergence at case 64 (slice
  deep600.json) in the FT-54 band exactly
  as Rounds 16 and 17 recorded it, zero
  file divergences, and the corpus guards
  stand exact at 371 and 6,284. No cell
  convicts and no repair is owed.
- FT-97. The realistic-volume pass. A
  synthetic corpus built for this round
  and measured from itself: 21,035
  session lines, 7,736,621 bytes, 27
  sessions. One long-haul session file of
  6,003 lines (two system lines and one
  foreign-session line among them,
  counted as skipped) converts to 7,500
  events, its
  totals standing at 750,000 input
  tokens, 135,000 output tokens, 748,500
  cache-read tokens, 1,500 tool calls,
  and 4,500 messages. A day of 24
  sessions carries 14,728 lines. One
  deep and wide session carries a tool
  payload nested 122 deep, an object of
  500 keys with astral-prefixed and
  U+E000-prefixed keys among them, a text
  of near 200 KB, and float-edge leaves.
  One dirty session of 300 lines carries
  an absolute path and a
  credential-shaped synthetic string
  through its texts, inputs, and results.
  Nineteen cells run over both
  implementations, the Python oracle and
  the shipped binary, and every cell
  agrees byte for byte in stdout, stderr,
  exit code, and written files: convert,
  digest, verify, validate, play, rollup,
  and compare over the long-haul tape;
  slice across sequences 2000 to 2500 and
  the digest of the slice; convert-dir
  over the day, its 25-file output tree
  identical on both sides; convert,
  digest, validate, and play over the
  deep and wide tape; convert and screen,
  to stdout and to a report file, over
  the dirty tape; and screen over the
  long-haul tape. The digest law holds
  across commands at this volume: the
  digest the digest command prints is the
  digest the verify command accepts, on
  both implementations.
- FT-98. The pin check. The fleet
  consumes canonical form 1 pinned at
  commit
  74a83298aeb31c82e62b9fb0c613f5bc8600799e,
  the Round 13 merge, whose tree
  (afd4d7d13520411e003762548fc79082a1b9cf6e)
  was extracted and hash-verified blob
  for blob, 50 files, before its binary
  was built. Thirteen cells run the pin
  binary and this round's binary over the
  same inputs, and the two agree byte for
  byte in stdout, stderr, exit code, and
  written files on every cell: convert,
  digest, validate, play, verify,
  compare, rollup, slice, and screen over
  the golden fixture, the golden digest
  printed identically by both, and four
  digest probes, the U+E000 and U+10000
  divergence pair, the shared astral
  prefix pair, float edges holding the
  smallest subnormal and the largest
  finite double, and a nested mixed
  object. The format the fleet consumes
  pinned is the format the head ships, at
  every surface this check touches.
- FT-99. This round's verification runs
  on the final tree. Cargo test 247
  passed, cargo clippy --all-targets with
  warnings denied clean, cargo fmt
  --check clean, and the codepoint scan
  over every tracked file at U+2014=0 and
  U+2013=0. The round changes no behavior:
  its whole delta is this contract's
  appends and the README rounds line, and
  the measurements at FT-96 to FT-98 were
  taken on a tree whose source is
  identical to the final tree's.
- FT-100. README records 18 of 100 only
  in the closure commit. The Cargo.toml
  description stands as FT-83 recorded
  it, a third variant awaiting the
  line-wide pass, unrepaired here by that
  item's own assignment.
- FT-101. The CodeRabbit disposition for
  this round's own pull request discharges
  here, on the read-back of PR 19 taken on
  2026-11-10. The pull request carries no
  reviews and no review comments, and its
  shell (6775748471) is a completed
  walkthrough over the final head
  5837edfa: no actionable comments were
  generated, Merge Risk Minimal, and
  pre-merge checks 5 passed, Description,
  Title, Docstring Coverage, Linked
  Issues, and Out of Scope. The Docstring
  Coverage row passes with no eligible
  functions found in the changed files,
  this round's diff being documentation
  only, so no dismissal is owed on it.
  Nothing stands to fix and nothing to
  dismiss. This item lands after the
  closure commit by the server-side
  sequence this round used, and the
  rounds line the closure set is
  untouched and still exact.
- FT-102. The read-back of PR 19, taken
  whole in this round before any other
  work, through the API: the reviews
  list, the review comments list, the
  issue comments list, and the review
  threads, every body read to its end.
  The full record is no reviews, no
  review comments, no review threads,
  and four issue comments. The final
  head's identity stands verified
  through the API in FT-101's place:
  a885977ebe931987cbe2bcffc4db4f84bde5d0bb,
  whose tree is
  5b9e0ca995559ae6afe9afad8a977fde9a441f4b,
  and the merge that closed the pull
  request (8ac81952, parents cefd6047
  and a885977e) carries the identical
  tree. The four issue comments are the
  owner's trigger (6775748199,
  @coderabbitai review), the shell
  (6775748471), the bot's auto-reply
  (6775849828, the action-not-completed
  form stating the last commit was
  already reviewed), and the owner's
  landing comment (6776560381), which
  records the merge, the round's gate
  (specification PASS with no findings,
  standards APPROVE with one Minor, the
  red team APPROVE with the same Minor),
  and the charge this item discharges.
  The shell stands in its final form
  exactly as follows. A rate-limit
  banner heads it (Review limit reached,
  run 4778027c-2869-4bc5-b390-4e2e34e46c89)
  over the amendment range 5837edfa to
  a885977e, files selected for
  processing CONTRACT.md alone, and the
  banner's own line that no actionable
  comments were generated in the recent
  review. Below the banner the first
  review's walkthrough stands retained
  (run c1814679-5e0c-4fe2-b698-a84d7a2821de,
  over cefd6047 to 5837edfa, files
  CONTRACT.md and README.md, its change
  assessment naming 5837edfa, effort 2
  Simple at about 10 minutes, Change
  Other). Merge Risk stands Minimal in
  the final form, its coverage naming
  5837edfabb4504bb319fa84a717efa5f2a724087
  as both the source and the covered
  commit, kind reviewed, the changes
  documentation only with no identified
  merge-blocking issue. The pre-merge
  checks in the final form stand at 4
  passed, Description, Title, Linked
  Issues, and Out of Scope, with no
  further rows. FT-101 stated the shell
  in its first form, a completed
  walkthrough over the final head
  5837edfa with pre-merge checks 5
  passed and a Docstring Coverage row
  among them; that form was superseded
  when the amendment push edited the
  shell, the Docstring Coverage row
  stands only in the superseded form,
  and the amendment range was never
  substantively bot-reviewed, the same
  standing form PR 18's shell reached.
  The disposition FT-101 recorded stands
  true under the final form, no
  actionable comments having been
  generated in either form, so nothing
  stood to fix and nothing to dismiss,
  and no bot follow-up corrects any
  disposition on the pull request. The
  pull request body as it finally stands
  carries the bot's release-notes
  appendix after the round's
  review-readiness block.
- FT-103. The canonical surfaces are
  measured fresh at the binary built
  from this round's base tree, the
  staged tree hash-verified blob for
  blob against the remote tree, 52
  blobs, no mismatches, before the
  build. The battery of FT-91 runs again
  in full: 54 cells, all 54 hold, the
  twelve key cells, the twenty eight
  float cells, and the fourteen
  stability cells over the golden
  fixture, which digests to
  cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3
  exactly, and FT-77's posture cell holds
  as contracted. The FT-56 differential
  harness runs against the Python oracle
  on this round's binary: 117 cases, one
  divergence at case 64 (slice
  deep600.json) in the FT-54 band exactly
  as Rounds 16, 17, and 18 recorded it,
  zero file divergences, and the corpus
  guards stand exact at 371 and 6,284.
  No cell convicts and no repair is owed.
- FT-104. The deepest joints FT-95 to
  FT-101 and the canonical form 1 items
  (FT-26 to FT-30, FT-39 to FT-43) leave
  open or unmeasured are probed at the
  same binary, against the Python oracle
  where the oracle can speak and against
  the contract's own text at the CLI
  seams: 24 cells, and every cell holds.
  Six verify-boundary cells: a digest
  padded with spaces verifies, and a
  digest padded with a tab, uppercased,
  and closed with a space verifies, the
  FT-30 amendment's whitespace and case
  law measured at the command; a
  truncated digest of 63 hex characters,
  an extended digest of 65, a non-hex
  digest of 64 characters, and an empty
  expected digest each refuse at exit 1
  with the mismatch named, the tape's
  own digest printed in the refusal.
  Six digest-rendering cells: the
  integers 2 to the 64th, a 30-digit
  integer, and minus 2 to the 63rd minus
  1 each digest to the oracle's digest,
  so integer rendering carries no
  machine-word wall at those points; a
  value nested in arrays 1,000 deep
  digests to the oracle's digest and
  1,001 deep refuses at exit 1 as nested
  beyond the interpreter limit, the
  FT-28 and FT-54 boundary standing at
  exactly that depth for arrays; a
  1e400 literal refuses at exit 1 as a
  non-finite number, nothing printed;
  and a string holding U+0001, U+001F,
  U+007F, U+2028, U+2029, U+FFFE, and
  U+FFFF digests to the oracle's digest.
  Five convert-dir seam cells: a
  subdirectory named with the session
  extension counts as failed with the
  error not a file and the run exits 1
  while its sibling converts; an
  uppercase-extension session file is
  passed over whole, counted nowhere,
  and the run exits 0; an empty source
  directory exits 0 with a summary of
  zero converted, zero failed, zero
  events, and no files; a mixed
  directory fails its unparseable
  session with the defect that events
  must hold at least one event, passes
  its non-session file over whole, and
  exits 1; and a session inside a nested
  subdirectory is not descended to, the
  run exiting 0 on the top-level session
  alone. Seven slice seam cells: a range
  wholly beyond the tape refuses at exit
  1 that the selection matches no events
  and writes nothing; a --from above its
  --to refuses naming both bounds; a
  negative bound refuses naming the
  bound's value; a slice with both
  bounds omitted writes the whole tape
  to stdout, its slice_of recording the
  golden digest with from, to, and kinds
  null and the full event count; a slice
  of a slice records the immediate
  source's digest, not the original's;
  a kind outside the enum refuses as an
  unknown event kind naming the kind,
  and a repeated kind filter selects
  exactly that kind; and a kind filter
  matching no events refuses as the
  empty selection FT-39 names. No cell
  convicts and no repair is owed.
- FT-105. The realistic-volume pass. The
  synthetic corpus of FT-97 is rebuilt
  and measured from itself again: 21,035
  session lines, 7,736,621 bytes, 27
  sessions, the long-haul session
  converting to 7,500 events with totals
  750,000 input tokens, 135,000 output
  tokens, 748,500 cache-read tokens,
  1,500 tool calls, and 4,500 messages,
  the deep session's payload nested 122
  deep and its widest object at 500
  keys. Nineteen cells run over both
  implementations, the Python oracle and
  the shipped binary, and every cell
  agrees byte for byte in stdout,
  stderr, exit code, and written files,
  over the same surfaces FT-97 names.
  The digest law holds across commands
  at this volume: the digest the digest
  command prints is the digest the
  verify command accepts, on both
  implementations.
- FT-106. This round's verification runs
  on the final tree. Cargo test 247
  passed and 0 failed, cargo clippy
  --all-targets with warnings denied
  clean, cargo fmt --check clean, and
  the codepoint scan over every tracked
  file at U+2014=0 and U+2013=0. The
  round changes no behavior: its whole
  delta is this contract's appends and
  the README rounds line, and the
  measurements at FT-103 to FT-105 were
  taken on a tree whose source is
  identical to the final tree's.
- FT-107. README records 19 of 100 only
  in the closure commit. The Cargo.toml
  description stands as FT-83 recorded
  it, a third variant awaiting the
  line-wide pass, unrepaired here by that
  item's own assignment.
- FT-108. A completion and correction
  item, landing on this round's own branch
  before its pull request opens. First,
  the shell length FT-102's charge
  required: the shell (6775748471) in its
  final form is 13,022 characters long,
  counted on the body as the API returns
  it. Second, the tape_version joint,
  probed by name on both arms at the same
  binary and oracle: validate accepts the
  version string 1.0, and refuses 1.1,
  2.0, 1, the empty string, and a missing
  tape_version, each refusal naming the
  field in the form tape_version must be
  1.0 with the value found, and the two
  arms agree on every probe in exit code,
  stdout, and stderr; digests over the
  same probe tapes agree across the arms
  as well. Third, the deepest cell of
  FT-104 restated from a fresh
  measurement: inside a tape whose event
  object wraps the value, the shipped
  binary digests arrays nested 997 deep
  and refuses at 998, so its boundary
  stands at exactly total depth 1,000
  digesting and 1,001 refusing, as FT-104
  claims. The Python oracle's command line
  accepts the same tape shape to 991
  arrays and refuses at 992 under its own
  interpreter limit, and the two arms
  print identical digests at every depth
  both can reach, so the split band
  between 992 and 997 is the FT-54
  interpreter-limit band and no new
  divergence; FT-104's clause that the
  1,000-deep value digests to the oracle's
  digest stands corrected on that
  sub-clause only, the oracle being unable
  to speak at that depth through its
  command line, and the cell rests on the
  contract's own text as FT-104's preamble
  provides. No code defect stands
  convicted by any of these measurements,
  and no repair to code is owed.
- FT-109. The CodeRabbit disposition for
  this round's own pull request discharges
  here, on the read-back of PR 20 taken on
  2026-11-11. The pull request carries one
  review (5862113756, COMMENTED, on the
  closure head b82e1df2, actionable
  comments posted: 1), one review comment,
  and three issue comments: the owner's
  trigger (6801394410), the shell
  (6801394975), and the bot's
  action-not-completed notice (6801563796,
  review rate limited, in the standing
  form). The shell stands at 12,216
  characters in its rate-limit form over
  the range b82e1df2 to b3565a87, the
  first review's walkthrough retained
  below the banner, Merge Risk Low, and
  pre-merge checks 5 passed, Description,
  Title, Docstring Coverage, Linked
  Issues, and Out of Scope, the Docstring
  Coverage row passing with no eligible
  functions found in the changed files.
  The one actionable item (4540570231,
  Minor, on this contract) is valid and
  stands repaired here: it holds that
  FT-106's verification record overstates
  the codepoint scan's scope, and it is
  right. The repository's
  scripts/codepoint_scan.py scans Markdown
  and text files only (the .md and .txt
  suffixes, .git excluded), not every
  tracked file, and FT-106's phrase over
  every tracked file stands corrected to
  that scope. The counts themselves stand
  as recorded, and this round's own Python
  scan over all 52 blobs of the final
  tree, source files included, counts
  U+2014=0 and U+2013=0 as well, so the
  gate's substance is unchanged and only
  its scope statement was wrong. Nothing
  else stands to fix and nothing stands to
  dismiss. This item lands after the
  FT-108 commit by the server-side
  sequence this round used, and the rounds
  line the closure set is untouched and
  still exact.
- FT-110. The correction record for this
  round's step 0 item and the repair record
  for its reviews. FT-109 as first written
  contracted the census of this pull request
  at three issue comments (6801394410,
  6801394975, 6801563796) and the walkthrough
  shell (6801394975) at 12,216 characters
  over the range b82e1df2 to b3565a87, with
  five pre-merge checks passed and a
  Docstring Coverage row standing. Both
  statements were superseded after the
  FT-109 commit landed, by the second
  review trigger the lane posted after it
  (6801678371) and the bot's
  action-not-completed notice answering it
  (6801687652). The census as it stands at
  the FT-109 head e86c3c22 is one review,
  one review comment, and five issue
  comments. The shell as it stands at that
  head is 11,353 characters, its recent
  review run f4d7e394 over the range
  b82e1df2 to e86c3c22, Merge Risk Low, the
  pre-merge checks counter at four passed
  (Description Check, Title check, Linked
  Issues check, Out of Scope Changes check)
  with no Docstring Coverage row standing
  in the form. FT-109's disposition stands
  unchanged. The Minor against FT-106's
  scan scope was valid, and its repair at
  FT-109 holds. The shell's form after
  this repair's head is charged to Round
  20's read-back, in the pattern FT-102
  set for a closed record. The reviews of
  the FT-109 head returned spec fidelity
  NOT PASS with two Majors, the two
  statements corrected here, standards
  APPROVE with no findings, and red team
  APPROVE with no findings.
- FT-111. A second correction and the
  union repair record. Two owner seats
  reviewed this round after a restart split
  the owner chat; FT-110 records one seat's
  set (spec NOT PASS with two Majors,
  standards APPROVE with no findings, red
  team APPROVE with no findings) and
  discharged the census and shell findings
  both sets share. The other seat's set
  returned spec fidelity NOT PASS with one
  Major and two Minors, standards NOT
  APPROVE with two Majors and three Minors,
  and red team APPROVE with one Minor, and
  the union of the two sets is this round's
  review record. Four findings of the
  second set stood outside FT-110's scope
  and are corrected here. FT-108's opening
  sentence says its item landed before the
  pull request opens; the pull request was
  created at 2026-11-11T16:31:57Z and the
  FT-108 commit is timestamped
  2026-11-11T16:35:53Z, so the item landed
  after the pull request opened, and the
  sentence stands corrected to that.
  FT-106's opening sentence says this
  round's verification runs on the final
  tree; the gates ran on the staged tree
  whose source is identical to the final
  tree's, as the pull request body's
  Verified block states, and the sentence
  stands corrected to that relationship.
  FT-104 says the value nested 1,001 deep
  refuses as nested beyond the interpreter
  limit; the refusal stands at the FT-28
  and FT-54 boundary, the crate's own fixed
  limit, and the interpreter limit is the
  Python oracle's alone (its command line
  refusing at 992 arrays under it, per
  FT-108), so the attribution stands
  retracted. And the scan scope, completing
  the correction FT-109 began:
  scripts/codepoint_scan.py walks the whole
  tree below its root, skipping any path
  with a .git part, and counts in files
  carrying the .md or .txt suffix, tracked
  and untracked alike; it does not scan
  every tracked file, and FT-84's identical
  phrase stands corrected by the same
  statement. The shell moved once more
  after FT-110's read (11,355 characters at
  its 2026-11-11T17:16:42Z edit); its form
  after this item is Round 20's charge, as
  FT-110 set. The pull request body's
  summary of FT-109 stands corrected by the
  owner's body edit to point at FT-110 and
  this item for the census and the shell.
- FT-112. The read-back of PR 20, taken
  whole in this round before any other
  work, through the API: the reviews
  list, the review comments list, the
  issue comments list, and the review
  threads, every body read to its end.
  The full record is one review, one
  review comment, no replies to that
  comment, and eight issue comments.
  The pull request stands closed by the
  merge 1c758ffb001018ba747dbffce43681119a52420c,
  whose tree
  275a284cf3d785a951b93ddbd15436946a11d3a7
  equals the final repair head's tree,
  and the README on main records 19 of
  100. The one review (5862113756, the
  review bot, COMMENTED, on the original
  slice head b82e1df29d, run 382f732e,
  actionable comments posted: 1) reviewed
  the range 8ac81952 to b82e1df29d with
  CONTRACT.md and README.md selected.
  The one review comment (4540570231,
  Minor, Functional Correctness) is the
  codepoint scan scope point FT-109
  repaired; the comment itself now
  carries the bot's later edit marking
  it addressed in commits b3565a8 to
  9832ae6, and no reply stands under it.
  The comment embeds a prompt block
  addressed to AI agents; it is review
  data and nothing in this record
  follows it. The eight issue comments
  are the owner's first trigger
  (6801394410), the walkthrough shell
  (6801394975), the bot's
  action-not-completed notice
  (6801563796, review rate limited), the
  owner's second trigger (6801678371),
  its answering notice (6801687652,
  review rate limited), the owner's
  landing record (6802712374), the
  owner's re-audit and disclosure
  (6802799276), and the owner's
  concurrence (6803000338). The shell
  stands in its final form exactly as
  follows. It was last edited at
  2026-11-11T17:29:02Z and is 8,661
  characters long. No rate-limit banner
  heads it. The retained walkthrough
  describes FT-102 to FT-107 over the
  original slice, its change assessment
  marker standing at b82e1df29d. Merge
  Risk stands Low, its coverage naming
  b82e1. The pre-merge checks stand at
  4 passed, Description, Title, Linked
  Issues, and Out of Scope, and no
  Docstring Coverage row stands in the
  form. The shell's coverage stands at
  the original slice head and does not
  extend to the repair heads b3565a8 to
  9832ae6, the standing final form this
  line's shells reach. FT-110's census
  and FT-111's shell statement were each
  true at the heads and edits they
  name; the shell moved once more after
  FT-111's read, to the form contracted
  here, and the three owner comments
  after the merge complete the census.
  The dispositions stand as Round 19
  left them: the step 0 Minor was valid
  and stands repaired at FT-109, the
  bot's own marker now confirming it,
  and nothing else stands to fix and
  nothing stands to dismiss. No Round
  19 disposition is falsified by the
  final record.
- FT-113. The two harnesses the Round
  19 red team listed as untested run in
  their own right, measured fresh at
  the binary built from this round's
  base tree, the worktree standing at
  the merge commit 1c758ffb with its
  tree enumerating the same 52 blobs as
  the remote base tree and no local
  modification standing, before the
  build. The battery of FT-91 runs in
  full as its two drivers: 40 cells in
  the first (the twelve key cells and
  the twenty eight float cells) and 14
  stability cells in the second, 54
  cells in all, and all 54 hold, the
  golden fixture digesting to
  cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3
  exactly, and FT-77's posture cell
  holding as contracted. The FT-56
  differential harness runs against the
  Python oracle on this round's binary:
  117 cases, one divergence at case 64
  (slice deep600.json) in the FT-54
  band exactly as Rounds 16, 17, 18,
  and 19 recorded it, zero file
  divergences, and the corpus guards
  stand exact at 371 and 6,284, the
  timestamp corpus and the number sweep
  counted in lines on this tree. No
  cell convicts and no repair is owed.
- FT-114. The depth law at the validate
  seam, probed at its exact points on
  both arms, in the wrapped shape FT-108
  used: a tape whose single event
  carries a payload wrapped in nested
  arrays. At digest, as the control, the
  binary digests the tape at 997 arrays
  and refuses at 998, FT-108's points
  exactly, and the oracle accepts at
  989 arrays and refuses at 990 in this
  shape, two arrays earlier than the
  991 and 992 FT-108 recorded on its
  shape, the oracle's point riding the
  tape's own structure as an interpreter
  limit does. At every depth both arms
  digest, spot-checked at 900 and 989
  arrays, the two print the identical
  digest. At validate the two arms
  split by seam. The shipped binary
  validates the tape at every probed
  depth, from 990 arrays through
  200,000 arrays, exit 0 throughout, no
  refusal and no crash: its reader and
  its validator's walks are uncapped in
  the whole band. The oracle validates
  the same tapes through 9,994 arrays
  and refuses at 9,995, exit 1, with the
  refusal wearing its not-valid-JSON
  form (defect, the file is not valid
  JSON), its reader reaching its own
  limit long after its digest renderer
  reaches its. FT-54's statement stands
  as written for the walks, which are
  uncapped on both arms; the asymmetry
  contracted here is in the oracle's
  reader, and it is the FT-54 band's
  character at a second seam. No cell
  convicts and no repair is owed.
- FT-115. The realistic-volume pass.
  The synthetic corpus of FT-97 is
  rebuilt and measured from itself
  again: 21,035 session lines,
  7,736,621 bytes, 27 sessions, the
  long-haul session converting to 7,500
  events with totals 750,000 input
  tokens, 135,000 output tokens,
  748,500 cache-read tokens, 1,500 tool
  calls, and 4,500 messages, the deep
  session's payload nested 122 deep and
  its widest object at 500 keys.
  Nineteen cells run over both
  implementations, the Python oracle
  and the shipped binary, and every
  cell agrees byte for byte in stdout,
  stderr, exit code, and written files,
  over the same surfaces FT-97 names.
  The digest law holds across commands
  at this volume: the digest the digest
  command prints is the digest the
  verify command accepts, on both
  implementations.
- FT-116. This round's verification runs
  on a tree whose source is identical
  to the final tree's. Cargo test 247
  passed and 0 failed, cargo clippy
  --all-targets with warnings denied
  clean, cargo fmt --check clean, and
  the codepoint scan stated at its true
  scope: the repository script counts
  U+2014=0 and U+2013=0 over the tree's
  Markdown and text files, and this
  round's own scan over all 52 blobs of
  the final tree, source files
  included, counts U+2014=0 and
  U+2013=0 as well. The round changes
  no behavior: its whole delta is this
  contract's appends and the README
  rounds line, and the measurements at
  FT-113 to FT-115 were taken on the
  base tree, whose source the final
  tree shares.
- FT-117. README records 20 of 100 only
  in the closure commit. The Cargo.toml
  description stands as FT-83 recorded
  it, a third variant awaiting the
  line-wide pass, unrepaired here by
  that item's own assignment.

- FT-118. The read-back of PR 21, taken
  whole in this round before any other
  work, through the API: the pull
  request body, the reviews list, the
  review comments list, and the issue
  comments list, every body read to its
  end. The pull request stands closed by
  the merge
  516a8f608399f9039ea81e5632bbaa17b97a1c02,
  whose tree
  70977b5f6386caad9a94dc86a8e037b8a9e3f8a2
  equals the closure head
  6b3008f040dd3026fa783a70cfd8195b4edc2be7's
  tree, and the README on main records
  20 of 100. The body stands in its
  final form at 5,169 characters, last
  edited 2026-11-12T00:19:47Z, carrying
  the owner's repair of its FT-114
  summary to the scoped form, identical
  digests at the two spot-checked
  depths, 900 and 989 arrays, and its
  process note on the first push stands
  confirmed by the record, the re-landed
  heads' trees equal to the local trees
  and the merge tree equal to the
  closure tree. The census is zero
  reviews, zero review comments, and
  five issue comments: the walkthrough
  shell (6810035910), the owner's
  trigger (6810039054), the bot's
  action-not-completed auto-reply
  (6810103385, already reviewed the last
  commit), the owner's step 0 account
  (6810294006), and the owner's landing
  comment (6810542335). The shell stands
  in its final form exactly as follows.
  It was created at
  2026-11-11T23:49:24Z, last edited at
  2026-11-12T00:06:27Z, and is 11,119
  characters long. No rate-limit banner
  heads it. Its walkthrough stands over
  the whole range, base 1c758ffb to the
  closure head 6b3008f0, its change
  assessment marker standing at the
  closure head itself, and its files
  selected are CONTRACT.md and
  README.md. Merge Risk stands Minimal,
  its coverage naming 6b300, the closure
  head. The pre-merge checks stand at 4
  passed, Title, Linked Issues, Out of
  Scope Changes, and Description, and no
  Docstring Coverage row stands in the
  form. Its own line stands: no
  actionable comments were generated in
  the recent review. The owner's step 0
  account read the shell at 11,117
  characters; the shell's final edit, at
  2026-11-12T00:06:27Z, added two
  characters and changed nothing the
  dispositions rest on, the checks, the
  Merge Risk, the coverage, and the
  no-actionable-comments line standing
  as the account recorded them. The
  dispositions stand as Round 20 left
  them and are confirmed against the
  closed record: the bot reviewed on
  open, no actionable comments stood,
  nothing stood to fix and nothing stood
  to dismiss, and no Round 20
  disposition is falsified.

- FT-119. FT-114's attribution, restated
  on fresh measurement. The probes are
  tapes built by string construction, a
  single tool_call event whose input
  carries the payload, the counted unit
  the nested arrays wrapping the scalar
  1 inside the input. At zero added
  wraps the input is the arrays
  themselves; at one, two, and three
  added wraps the arrays sit inside that
  many nested single-key objects inside
  the input. Each arm's digest point is
  found by binary search and stated as
  the last depth digested and the first
  refused. The shipped binary: 997 and
  998 at zero wraps, 996 and 997 at one,
  995 and 996 at two, 994 and 995 at
  three, its own fixed render depth law
  counting every container on the path,
  object or array. The Python oracle by
  the fleet standard runner, python3 -c
  importing flighttape.cli's main with
  the oracle source on PYTHONPATH: 991
  and 992 at zero wraps, 990 and 991 at
  one, 989 and 990 at two, 988 and 989
  at three. The same oracle by the
  module route, python3 -m
  flighttape.cli on the same PYTHONPATH:
  989 and 990 at zero wraps, 988 and 989
  at one, 987 and 988 at two, 986 and
  987 at three. Twelve point
  determinations, and two laws stand out
  of them. First, each added container
  level moves every arm's point by
  exactly one array, the binary's
  included: the tape's own structure
  consumes the depth budget one level
  for one level. Second, on identical
  files the oracle's two invocation
  routes stand exactly two arrays apart
  at every level, the module route
  reaching its limit two arrays earlier:
  the invocation baseline consumes
  headroom the standard runner leaves
  free. The oracle's digest point is
  therefore interpreter-headroom
  relative, consumed by the tape's
  structure and by the invocation route
  alike, and no figure for it stands in
  this contract without its route beside
  it. FT-108's 991 and 992 stand as the
  fleet standard runner's points on its
  shape, and FT-114's 989 and 990 stand
  as the module route's points on its
  shape; both rounds measured truly, and
  FT-114's clause, the oracle's point
  riding the tape's own structure as an
  interpreter limit does, stands
  corrected to the two-consumer
  statement above. Every refusal in the
  grid wears the one form, exit 1, the
  tape is nested beyond the interpreter
  limit, the binary's wording mirroring
  the oracle's by the port's design. No
  cell convicts and no repair is owed.

- FT-120. FT-114's digest identity,
  contracted in its scoped form on fresh
  measurement. On the zero-wrap tapes of
  FT-119, the three arms print the
  identical digest at exactly the
  spot-checked depths 100, 500, 900,
  950, and 989 arrays, the printed
  digests compared byte for byte, five
  depths, three arms each, no
  divergence. At 990 and 991 arrays the
  binary and the fleet standard runner
  print the identical digest and the
  module route refuses; at 997 the
  binary alone digests, both oracle
  routes refusing. The bands at zero
  wraps stand therefore, acceptance
  stated apart from agreement: every
  arm digests to 989, agreement
  holding only at the measured depths
  named above; at 990 and 991 the
  binary and the standard runner
  digest and, both depths measured,
  agree; from 992 to 997 the binary
  digests alone; at 998 no arm digests.
  Identity is asserted at
  exactly the depths named here and at
  no others: where two arms digest the
  same tape they print the same digest
  at every depth this round checked, and
  the invocation route moves only where
  the refusal falls, never the digest
  printed. FT-114's sentence, at every
  depth both arms digest, spot-checked
  at 900 and 989 arrays, the two print
  the identical digest, stands replaced
  by this item's enumeration.

- FT-121. FT-114's validate probe set,
  named as a set on fresh measurement.
  The shipped binary validates the
  zero-wrap tapes of FT-119 at exactly
  these eighteen depths, exit 0 at every
  one, no refusal and no crash: 990,
  991, 992, 997, 998, 999, 1000, 1001,
  2000, 5000, 9994, 9995, 10000, 20000,
  50000, 100000, 150000, and 200000
  arrays. Its reader and its validator's
  walks are uncapped across the whole
  named set. The oracle's reader point
  at the same seam, found by binary
  search on the same tapes: by the fleet
  standard runner it accepts at 9,994
  arrays and refuses at 9,995; by the
  module route it accepts at 9,991 and
  refuses at 9,992; the two routes stand
  three arrays apart at the reader seam,
  against the two of FT-119's digest
  seam. The structure law of FT-119
  holds at this seam as well: on the
  standard route each added wrap moves
  the reader point exactly one array,
  9993 and 9994 at one wrap, 9992 and
  9993 at two, 9991 and 9992 at three,
  and the module route at one wrap
  stands at 9990 and 9991. The refusal
  on both routes wears the reader's
  form, exit 1, defect, the file is not
  valid JSON. FT-114's 9,995 stands as
  the fleet standard runner's point.
  FT-114's sentence, the binary
  validating at every probed depth from
  990 through 200,000 arrays, now stands
  beside this item's named set, the
  eighteen depths this round probed; the
  two sets are not claimed to be the
  same set, Round 20's probe log being
  lost.

- FT-122. The realistic-volume pass, the
  golden fixture, and the fleet pin. The
  synthetic corpus of FT-97 is rebuilt
  with a new seed, 20261112, Round 20's
  having been 20261110, and measured
  from itself: 21,035 session lines,
  7,736,621 bytes, 27 sessions, the deep
  session's payload nested 122 deep and
  its widest object at 500 keys, the
  long-haul session converting to 7,500
  events with totals 750,000 input
  tokens, 135,000 output tokens, 748,500
  cache-read tokens, 1,500 tool calls,
  and 4,500 messages. The shape figures
  stand exactly as FT-115 recorded them
  under the other seed, and the content
  moved with the seed: the long-haul
  tape digests to
  8b1d63f93938c654eb682ce48474c247d27c638bc4800522eb74ed7502988b4c,
  against Round 20's
  658d692144781141c77267f959c5f24b38dce2ca26453b860254943cb10d64e9.
  Nineteen cells run over both
  implementations, the Python oracle by
  the fleet standard runner and the
  shipped binary, and every cell agrees
  byte for byte in stdout, stderr, exit
  code, and written files, over the same
  surfaces FT-97 names. The digest law
  holds across commands at this volume:
  the digest the digest command prints
  is the digest the verify command
  accepts, on both implementations. The
  golden fixture: converting
  tests/fixtures/claude_session_synthetic.jsonl
  on both implementations writes
  byte-identical tapes of 1,663 bytes,
  and the binary and both oracle routes
  print the golden digest
  cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3
  for it. The fleet pin stands
  re-confirmed against the live commit
  74a83298aeb31c82e62b9fb0c613f5bc8600799e:
  its tree is
  afd4d7d13520411e003762548fc79082a1b9cf6e,
  whose recursive listing counts 58
  entries, 50 of them blobs, as FT-98
  recorded.

- FT-123. This round's verification runs
  on a tree whose source is identical to
  the final tree's. Cargo test 247
  passed and 0 failed, cargo clippy
  --all-targets with warnings denied
  clean, cargo fmt --check clean, and
  the codepoint scan stated at its true
  scope: the repository script counts
  U+2014=0 and U+2013=0 over the tree's
  Markdown and text files, and this
  round's own scan over all 52 blobs of
  the final tree, source files included,
  counts U+2014=0 and U+2013=0 as well.
  The round changes no behavior: its
  whole delta is this contract's appends
  and the README rounds line, and the
  measurements at FT-119 to FT-122 were
  taken on the base tree, whose source
  the final tree shares.

- FT-124. README records 21 of 100 only
  in the closure commit. The Cargo.toml
  description stands as FT-83 recorded
  it, a third variant awaiting the
  line-wide pass, unrepaired here by
  that item's own assignment.

- FT-125. The read-back of PR 22, taken
  whole in this round before any other
  work, through the API: the pull request
  body, the reviews list, the review
  comments list, and the issue comments
  list, every body read to its end. The
  pull request stands closed by the merge
  c9665721129ee5fcc75c5296bf3451c295880e62,
  whose tree
  b179fbd0e90ca7ed0ce6ec226dc4e19d7b8fc3b7
  equals the repair head
  97da7f93dc6615271ed45c65d4f2b7cecab8fa1d's
  tree, and the README on main records 21
  of 100. The body stands in its final
  form at 8,115 characters, the bot's
  release-notes appendix included, its
  heads named slice
  0c1cb74ebf30582dc871479f4fc2e997c6cba938,
  closure
  6d6c12ba639ab7005f3cda30bb9d04e90e561a67,
  and repair 97da7f93, and its point 7
  carrying the step 0 disposition and the
  staging disclosure in the owner's
  repaired form, the three commits. The
  census is three reviews, three review
  comments, and four issue comments. The
  reviews are the bot's COMMENTED review
  (5870479322, on the closure head
  6d6c12ba, run
  790e7ba9e72e4576b976c298e1bbe74d1,
  actionable comments posted: 1, the
  included allowance used and none
  remaining), the owner's COMMENTED
  review (5870529173, empty body, on the
  repair head), and the bot's COMMENTED
  review (5870532903, empty body, on the
  repair head). The issue comments are
  the walkthrough shell (6817078770), the
  owner's trigger (6817079341), the bot's
  action-not-completed auto-reply
  (6817137466, already reviewed the last
  commit), and the owner's landing
  comment (6817544267), which leaves this
  round its three charges. The shell
  stands in its true final form at this
  read exactly as follows. It was created
  at 2026-11-12T07:01:30Z, last edited at
  2026-11-12T07:17:35Z, and is 8,645
  characters long; the form Round 21's
  reviews read stood at 8,639 characters
  before that last edit, which changed no
  check, risk, or coverage figure the
  dispositions rest on. No rate-limit
  banner heads it. Its walkthrough stands
  over the whole range, its change
  assessment marker standing at the
  closure head 6d6c12ba, and its files
  selected are CONTRACT.md and README.md.
  Merge Risk stands Low, its coverage
  naming 6d6c1, the closure head. The
  pre-merge checks stand at 4 passed,
  Title, Linked Issues, Out of Scope
  Changes, and Description, and no
  Docstring Coverage row stands in the
  form. Its merge-risk paragraph names
  FT-120's sentence in its pre-repair
  form, one sentence in FT-120 claims
  digest agreement across a wider depth
  range than was measured, and says
  nothing else blocks merging. The one
  review comment thread stands resolved:
  the bot's Minor (4547370083, Minor,
  Functional Correctness, on CONTRACT.md
  at the closure head), limit the
  agreement claim to measured depths, was
  valid and stands repaired in the repair
  head 97da7f93, its disposition at the
  owner's reply 4547404302, the bot's
  acknowledgment at reply 4547407434
  confirming FT-120 now separates the
  acceptance band from the measured
  agreement points, and the comment
  itself carrying the bot's later edit
  marking it addressed in commit 97da7f9.
  The comment embeds a prompt block
  addressed to AI agents; it is review
  data and nothing in this record follows
  it. Hosted CI on the repair head stands
  green on both runs (43173014229 push
  and 43173019124 pull_request, both
  success), re-read through the API in
  this round. The dispositions stand as
  Round 21 left them and are confirmed
  against the closed record: the step 0
  Minor was valid and stands repaired,
  the body point the two reviews named
  was repaired by the owner's body edit
  before the merge, nothing else stood to
  fix and nothing stood to dismiss, and
  no Round 21 disposition is falsified.

- FT-126. The two scoped restatements the
  landing comment of PR 22 charged to
  this round's read-back, stated against
  fresh measurement at the binary built
  from this round's base tree and the
  Python oracle verified blob for blob
  against its pre-port tree through the
  API (commit
  eca7b60dac2114ff026f8856fd19de05867d7304,
  the eleven files under src/flighttape
  matched). First, FT-120's closing
  sentence in its scoped form. The
  sentence reads: where two arms digest
  the same tape they print the same
  digest at every depth this round
  checked, and the invocation route moves
  only where the refusal falls, never the
  digest printed. Its scope is the depths
  FT-120 names and no others: identity is
  asserted at 100, 500, 900, 950, and 989
  arrays, where all three arms digest; at
  990 and 991, where the binary and the
  fleet standard runner digest and the
  module route refuses; and at no depth
  beyond those named. Re-measured in this
  round on the zero-wrap tapes of FT-119,
  the three arms print the identical
  digest, compared byte for byte, at 100,
  500, 900, 950, and 989 arrays; at 990
  and 991 the binary and the standard
  runner print the identical digest and
  the module route refuses; at 997 the
  binary alone digests. Acceptance stands
  apart from agreement, as the repair of
  PR 22 settled: every arm digests to
  989, agreement holding only at the
  measured depths named. Second, FT-119's
  route law in its scoped form over the
  historical figures. The law, measured
  on FT-119's own grid and re-measured in
  this round at FT-127, is that each
  added container level moves every arm's
  digest point by exactly one array and
  the oracle's two invocation routes
  stand exactly two arrays apart at every
  level. Over the figures the earlier
  rounds recorded without a route, the
  law governs by FT-119's assignment and
  not by re-measurement of those rounds'
  shapes: FT-108's 991 and 992 stand as
  the fleet standard runner's points on
  FT-108's shape, and FT-114's 989 and
  990 stand as the module route's points
  on FT-114's shape, FT-108 and FT-114
  carrying their figures in their own
  text without a route, as historical
  text. FT-114's route assignment rests
  on FT-119's reproduction, its grid
  measuring the module route at 989 and
  990 on the zero-wrap shape, and not on
  a surviving Round 20 invocation log,
  Round 20's probe set inside its band
  being nowhere enumerated and its joint
  logs lost, as FT-121 records. No
  earlier figure is reworded by this
  item.

- FT-127. The depth law re-measured fresh
  at both seams and both oracle routes,
  at wrap levels zero and four, the
  probes in FT-119's shape built by
  string construction at the binary built
  from this round's base tree, each arm's
  point found by binary search and stated
  as the last depth accepted and the
  first refused, the invocation route
  pinned beside every oracle figure. At
  the digest seam at zero wraps: the
  shipped binary 997 and 998; the oracle
  by the fleet standard runner 991 and
  992; the oracle by the module route 989
  and 990, FT-119's grid reproduced
  exactly. At four wraps: the binary 993
  and 994; the standard runner 987 and
  988; the module route 985 and 986, the
  figures the Round 21 red team's
  out-of-sample extension measured, now
  measured by the round itself. The two
  laws of FT-119 hold at both levels:
  each added container level moves every
  arm's point by exactly one array, four
  wraps standing exactly four below zero
  wraps on all three arms, and the
  oracle's two routes stand exactly two
  arrays apart at each level. Every
  digest refusal in the grid wears the
  one form on all three arms, exit 1, the
  tape is nested beyond the interpreter
  limit. At the validate seam: the
  shipped binary validates the zero-wrap
  tapes at exactly the eighteen depths
  FT-121 names, 990 through 200,000
  arrays, exit 0 at every one, its reader
  and its validator's walks uncapped
  across the named set. The oracle's
  reader point at zero wraps stands by
  the standard runner at 9,994 and 9,995
  and by the module route at 9,991 and
  9,992, the two routes three arrays
  apart at this seam, as FT-121 records;
  the structure law re-measured on the
  standard route at one, two, and three
  wraps (9,993 and 9,994; 9,992 and
  9,993; 9,991 and 9,992) and on the
  module route at one wrap (9,990 and
  9,991), FT-121's figures reproduced. At
  four wraps, measured in this round for
  the first time at this seam: the
  standard runner accepts at 9,990 and
  refuses at 9,991, the module route
  accepts at 9,987 and refuses at 9,988;
  each route's point stands exactly four
  arrays below its zero-wrap point, and
  the routes stand exactly three apart at
  this level as at zero. The validate
  refusal on both routes wears the
  reader's form, exit 1, defect, the file
  is not valid JSON. No cell convicts and
  no repair is owed.

- FT-128. The two harnesses run in full
  in their own right, measured fresh at
  the binary built from this round's base
  tree, the drivers the Round 20 lane
  banked in the workspace re-pointed at
  this round's binary and scratch. The
  battery of FT-91 runs as its two
  drivers: 40 cells in the first (the
  twelve key cells and the twenty eight
  float cells) and 14 stability cells in
  the second, 54 cells in all, and all 54
  hold, the golden fixture digesting to
  cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3
  exactly, and FT-77's posture cell
  holding as contracted. The FT-56
  differential harness runs against the
  Python oracle on this round's binary:
  117 cases, one divergence at case 64
  (slice deep600.json, exit 1 against 0)
  in the FT-54 band exactly as Rounds 16
  through 21 recorded it, zero file
  divergences, and the corpus guards
  stand exact at 371 and 6,284, the
  timestamp corpus and the number sweep
  counted in lines on this tree. No cell
  convicts and no repair is owed.

- FT-129. The realistic-volume pass, the
  golden fixture, and the fleet pin. The
  synthetic corpus of FT-97 is rebuilt
  with a new seed, 20261113, Round 21's
  having been 20261112, and measured from
  itself: 21,035 session lines, 7,736,621
  bytes, 27 sessions, the deep session's
  payload nested 122 deep and its widest
  object at 500 keys, the long-haul
  session converting to 7,500 events with
  totals 750,000 input tokens, 135,000
  output tokens, 748,500 cache-read
  tokens, 1,500 tool calls, and 4,500
  messages. The shape figures stand
  exactly as FT-115 and FT-122 recorded
  them under the other seeds, and the
  content moved with the seed: the
  long-haul tape digests to
  f9e23830070deba9a7e5dd4f31ec213803bedf4609b85ef6bc5c12a50bf456d2,
  against Round 21's
  8b1d63f93938c654eb682ce48474c247d27c638bc4800522eb74ed7502988b4c
  and Round 20's
  658d692144781141c77267f959c5f24b38dce2ca26453b860254943cb10d64e9.
  Nineteen cells run over both
  implementations, the Python oracle by
  the fleet standard runner and the
  shipped binary, and every cell agrees
  byte for byte in stdout, stderr, exit
  code, and written files, over the same
  surfaces FT-97 names. The digest law
  holds across commands at this volume:
  the digest the digest command prints is
  the digest the verify command accepts,
  on both implementations. The golden
  fixture: converting
  tests/fixtures/claude_session_synthetic.jsonl
  on both implementations writes
  byte-identical tapes of 1,663 bytes,
  and the binary and both oracle routes
  print the golden digest
  cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3
  for it. The fleet pin stands
  re-confirmed against the live commit
  74a83298aeb31c82e62b9fb0c613f5bc8600799e:
  its tree is
  afd4d7d13520411e003762548fc79082a1b9cf6e,
  whose recursive listing counts 58
  entries, 50 of them blobs, read back
  through the API in this round.

- FT-130. The owed workspace copies of
  Round 21's scratch, charged by PR 22's
  landing comment, stand discharged as
  unrecoverable and are not
  reconstructed. At this round's read,
  /tmp/ft21 does not exist: the scratch
  the Round 21 lane staged there (its
  progress file, probe.py and probe2.py,
  results, volume21.py,
  items-wrapped.txt, and gate logs) was
  wiped with /tmp after that round
  closed, and the workspace directory
  that stood ready for the copies,
  ~/workspace/retro-line/ft21-scratch/,
  stands empty. No copy can be made from
  a source that no longer stands, and no
  instrument is rebuilt under this item,
  a rebuilt instrument being a new
  measurement rather than a copy. The
  figures the scratch held stand in
  FT-118 to FT-124 and in PR 22's body
  and landing record, which are that
  round's durable record; this round's
  own instruments stand in its lane
  scratch and are named in its pull
  request body.

- FT-131. This round's verification runs
  on a tree whose source is identical to
  the final tree's. Cargo test 247 passed
  and 0 failed, stated with its method:
  the suite ran target by target (the
  library's 7, then cli 22, compare 19,
  convert 34, digest_cli 5, identity 13,
  json 8, play 17, redaction 12,
  robustness 15, rollup 22, scale 3,
  schema 5, screen 15, slice 24,
  timestamps 1, and validate 25, with doc
  tests at 0), the counts summing to 247,
  because a single parallel invocation
  filled the shared temporary filesystem
  mid-link and the workspace target
  directory stood refused by the
  file-count quota; every target's own
  result line stands in the lane's gate
  log. Cargo clippy --all-targets with
  warnings denied clean, cargo fmt
  --check clean, and the codepoint scan
  stated at its true scope: the
  repository script counts U+2014=0 and
  U+2013=0 over the tree's Markdown and
  text files, and this round's own scan
  over all 52 blobs of the final tree,
  source files included, counts U+2014=0
  and U+2013=0 as well. The round changes
  no behavior: its whole delta is this
  contract's appends and the README
  rounds line, and the measurements at
  FT-126 to FT-129 were taken on the base
  tree, whose source the final tree
  shares.

- FT-132. README records 22 of 100 only
  in the closure commit. The Cargo.toml
  description stands as FT-83 recorded
  it, a third variant awaiting the
  line-wide pass, unrepaired here by that
  item's own assignment.

- FT-133. Two corrections to this
  round's own items, appended on the
  owner's reviews, the originals
  standing. First, FT-130's closing
  clause. It says this round's own
  instruments stand in its lane scratch
  and are named in its pull request
  body. Both halves are wrong on this
  round's record: the build lane was
  interrupted by a box restart after
  banking the round's four commits, its
  scratch, instruments, and progress
  file were lost with /tmp, as the pull
  request body's disclosure states, and
  the body names no instruments or
  scratch path. The round's durable
  record is this contract and the pull
  request record. FT-130's discharge
  itself stands as written. Second,
  FT-126's verification channel. It
  says the Python oracle was verified
  blob for blob against its pre-port
  tree through the API, at commit
  eca7b60dac2114ff026f8856fd19de05867d7304.
  The blob-for-blob fact is true and
  was re-verified by the red team, but
  the channel does not reproduce as
  stated: that commit is not reachable
  through the GitHub API in this
  repository, and the verification
  stands through the local clone's
  object store, whose history carries
  the commit. No measurement in either
  item is touched by these corrections.

- FT-134. Round 22's pull request 23 read
  back whole on its closed record through
  the GitHub API: zero reviews, zero review
  comments, and four issue comments, the
  trigger 6834513671 the owner posted, the
  shell 6834515668, the auto-reply
  6834551159 (573 characters, already
  reviewed), and the landing comment
  6835143290 (2,262 characters). The Round
  22 spec note for this read-back, and the
  owner's brief for this round after it,
  both carried the census as three reviews
  and three review comments; those figures
  are pull request 22's, contracted at
  FT-125, and the correction stands here on
  the record rather than in a summary: pull
  request 23's census on its closed record
  is zero, zero, and four. The shell stands
  in its true final form at this read at
  11,202 characters, created
  2026-11-12T20:02:06Z and last edited
  2026-11-12T20:24:46Z, a completed review
  and not a skip: a walkthrough over FT-125
  to FT-132; Merge Risk Minimal up to 7d616,
  the closure head
  7d61681d74d31ed373ea5fa6d35166550cf48e15,
  with the change assessment marked at that
  head, the commits reviewed c9665721 to
  7d61681d, and the files selected for
  processing CONTRACT.md and README.md; the
  pre-merge checks at four passed, Title
  Check, Linked Issues Check, Out of Scope
  Changes Check, and Description Check, with
  none failed and none inconclusive;
  Docstring at zero occurrences; no
  rate-limit banner in the body; the line
  that no actionable comments were generated
  in the recent review standing in it; and
  Run ID
  15e9efc0-1f08-400b-8000-47d7c2cda6a5, with
  the included allowance used and zero
  remaining after it. The shell moved
  through four generations under that one
  comment ID: 11,196 characters in its first
  completed form, recorded
  2026-11-12T20:03:32Z; 11,198 at the edit
  of 2026-11-12T20:06:37Z; 13,900 at the
  edit of 2026-11-12T20:23:09Z with coverage
  at the repair head a4b6fab55; and the
  final edit at 2026-11-12T20:24:46Z
  standing at 11,202 in a closure-head
  generation, its coverage at the closure
  head only, neither repair head named. The
  owner's body restatement is bound to the
  third generation and is true at that
  moment on every figure it quotes: 13,900
  characters, the repair head named, the
  census at three issue comments at that
  edit, and the completed-review posture.
  The final edit superseded figures only and
  no disposition fact: no actionable comment
  stands in any generation, and the four
  passed checks, the Minimal risk, and the
  absent Docstring section hold in the first
  generation and the last alike. The landing
  comment records the final form already, so
  nothing in the Round 22 step 0 account is
  corrected here beyond the census figures
  above: the restatement stands,
  moment-bounded as it was written. Charge
  (b), FT-125's Run ID rendering: pull
  request 22's record carries its run ID
  hyphenated, in review 5870479322, as
  790e7ba9-e72e-4576-976c-298e1bbe74d1,
  thirty-two digits with four hyphens.
  FT-125 renders it
  790e7ba9e72e4576b976c298e1bbe74d1: the
  hyphens are dropped, as the Round 22 spec
  note said, and a further b stands at the
  seventeenth character where the record has
  none, making thirty-three characters
  against the record's thirty-two with the
  hyphens removed, so the rendering as
  printed identifies no run. The true
  hyphenated form stands in this item. The
  ID is a citation no tooling resolves and
  no disposition in any round rested on it,
  so the correction changes no disposition;
  it repairs the citation, and it stands as
  a correction of the record FT-125 made,
  its original text unrewritten. Charge (c):
  no late CodeRabbit finding stands on pull
  request 23. Nothing was posted there after
  the landing comment, the shell's last edit
  predates the merge, and the auto-reply's
  already reviewed is the last word the bot
  left, so nothing is owed a disposition. No
  disposition the Round 22 reviews made is
  falsified by the closed record: the step 0
  census and figures Minor was repaired by
  the owner's moment-bounded restatement,
  FT-125's and FT-126's contract Minors were
  repaired by FT-133 with both delta reviews
  clean, and the red team's Minor on
  FT-114's attribution stands answered by
  FT-127's re-measurement on FT-119's law.
  FT-83 stands open and unrepaired, assigned
  outside this round's scope to a line-wide
  pass, and is carried open by name here.

- FT-135. The depth law re-measured fresh on
  the binary built from Round 22's closing
  tree, at probes in FT-119's shape: tapes
  built by string construction, a single
  tool_call event whose input carries the
  payload, the counted unit the nested
  arrays wrapping the scalar 1 inside the
  input, and at W added wraps the arrays
  sitting inside W nested single-key objects
  inside the input. Every point stands found
  by binary search and stated as the last
  accepted input depth and the first
  refused. At zero added wraps the digest
  seam stands unmoved on all three arms: the
  binary digests at 997 and refuses at 998,
  the oracle by the fleet standard runner
  digests at 991 and refuses at 992, and the
  oracle by the module route digests at 989
  and refuses at 990. At four added wraps
  every arm stands exactly four below its
  zero-wrap point, FT-127's first
  measurements at that level reproduced: the
  binary at 993 and 994, the standard runner
  at 987 and 988, the module route at 985
  and 986. Every refusal on every arm wears
  the one form, exit 1 with the tape nested
  beyond the interpreter limit, so both of
  FT-119's laws hold at both levels on the
  closing tree: one array per added
  container level on every arm, and the
  oracle routes exactly two apart on the
  digest seam with the route named beside
  every interpreter-limit figure here.
  Identity holds in FT-126's scoped form
  over the same zero-wrap tapes: all three
  arms print the identical digest at 100,
  500, 900, 950, and 989; the binary and the
  standard runner print the identical digest
  at 990 and 991, where the module route
  already refuses; and at 997 the binary
  prints its digest alone, past both oracle
  routes' limits. No all-three identity is
  claimed past 989 and no
  binary-and-standard identity past 991, the
  exact scope FT-126 set. At the validate
  seam the binary again stands apart from
  the interpreter: it validates with exit 0
  at each of the eighteen named zero-wrap
  input depths, 990, 991, 992, 997, 998,
  999, 1000, 1001, 2000, 5000, 9994, 9995,
  10000, 20000, 50000, 100000, 150000, and
  200000, uncapped on this seam everywhere
  FT-121 looked and at the extremes again
  here. The oracle's JSON reader stops at
  its interpreter limit and its routes stand
  exactly three apart on this seam: at zero
  wraps the standard runner accepts 9,994
  and refuses 9,995 and the module route
  accepts 9,991 and refuses 9,992,
  reproducing FT-121's points; at four wraps
  each route stands exactly four below its
  zero-wrap point, the standard runner
  accepting 9,990 and refusing 9,991 and the
  module route accepting 9,987 and refusing
  9,988, FT-127's first measurements
  reproduced, the routes three apart at both
  levels. Every oracle refusal on this seam
  wears the reader's form, exit 1 with a
  defect reporting the file is not valid
  JSON.

- FT-136. The FT-91 battery and the FT-56
  differential run in full against the
  binary built from Round 22's closing tree.
  The battery passes 54 of 54 cells over its
  two named drivers: the original driver's
  40 cells over canonical key order and
  float rendering, and battery2.py's 14
  cells over stability, determinism, and
  edge cases, the golden digest standing in
  battery2.py's convert-dir cell and the
  FT-77 posture cell holding its contracted
  shape. The total is stated from those
  named parts, 40 and 14, and from no wider
  run. The differential runs its 117 cases
  with a single divergence at case 64, slice
  over deep600.json, exit 1 against 0 in the
  FT-54 band where the binary's own depth
  refusal is the contracted behaviour, the
  output divergence riding on that exit gap,
  and zero file divergences over the 117
  cases. Its corpus guards stand exact on
  the closing tree, counted in lines on this
  tree: tests/fixtures/timestamp_corpus.txt
  at 371 lines and
  tests/fixtures/number_sweep.txt at 6,284
  lines. The oracle both harnesses measured
  against stands verified at FT-138 through
  the channel named there.

- FT-137. The FT-97 volume pass runs at a
  new seed, 20261115, Round 22's having been
  20261113, the same shape the generator has
  held since Round 20 coming out of a fresh
  draw: the corpus measures from itself at
  21,035 lines, 7,736,621 bytes, and 27
  sessions, the deep payload reaching 122
  containers and the widest object holding
  500 keys, and the long-haul session
  carrying 7,500 events with run totals of
  750,000 input tokens, 135,000 output
  tokens, 748,500 cache-read tokens, 1,500
  tool calls, and 4,500 messages. All 19
  cells stand byte-identical over both
  implementations with no failures: convert,
  digest, verify, validate, play, rollup,
  self-compare, and slice across the
  long-haul session; digest over the window
  counts; the directory and tree
  conversions; the deep-and-wide tape's
  convert, digest, validate, and play; the
  dirty tape's convert and its two screen
  cells; and the long-haul screen. The
  long-haul digest is the corpus's own under
  this seed,
  47c56b707f65b71c144ab98df8455fa41cac58202f371a226eb6185ad7562c25,
  and differs from Round 22's as the new
  draw requires. The golden fixture executes
  byte for byte: converting
  tests/fixtures/claude_session_synthetic.jsonl
  writes tapes of 1,663 bytes,
  byte-identical on the binary and on the
  standard runner, and the binary and both
  oracle routes print digest
  cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3
  for it. The fleet pin re-confirms live
  through the API: the commit
  74a83298aeb31c82e62b9fb0c613f5bc8600799e
  resolves, its tree
  afd4d7d13520411e003762548fc79082a1b9cf6e
  lists 58 entries untruncated, and exactly
  50 of them are blobs.

- FT-138. The gate run against Round 22's
  closing tree. Cargo test passes 247 tests
  with zero failures, the count stated
  target by target because a single parallel
  invocation fills tmpfs mid-link on this
  box: lib 7, cli 22, compare 19, convert
  34, digest_cli 5, identity 13, json 8,
  play 17, redaction 12, robustness 15,
  rollup 22, scale 3, schema 5, screen 15,
  slice 24, timestamps 1, validate 25, and
  doc 0, the counts summing to 247 and the
  doc count a true zero. On this round the
  shared memory filesystem holding the
  target filled to 100 percent mid-gate,
  after the redaction target: the nine
  banked targets stood at their recorded
  counts, the regenerable volume work
  directory beside the target was deleted,
  each finished test binary was removed
  after its target ran, and the remaining
  eight targets ran clean to completion, no
  count in this item claimed from an
  interrupted run. Clippy over all targets
  with warnings denied exits 0, and cargo
  fmt in check mode exits 0. The codepoint
  scan stands at zero on both instruments:
  the repository's own
  scripts/codepoint_scan.py over the tree
  reports U+2014 at zero and U+2013 at zero,
  and this round's own Python scan over all
  52 blobs of the final tree reports the
  same two zeros, each counted by codepoint.
  The oracle the harnesses and volume pass
  measured against stands verified blob for
  blob against commit
  eca7b60dac2114ff026f8856fd19de05867d7304
  through the local clone's object store at
  ~/workspace/retro-line/flighttape: all 11
  files under that commit's src/flighttape
  match the working copy at
  ~/workspace/retro-line/ft15-scratch/oracle/src/flighttape
  blob for blob, with zero mismatches. The
  local clone's object store is the channel
  this verification used, stated exactly:
  that commit is not reachable through the
  GitHub API in this repository, as FT-133
  records, and no part of this item claims
  the API channel for it.

- FT-139. Round 23 closes. Its record stands
  at FT-134 to FT-139: the Round 22
  read-back, with the census correction and
  the Run ID correction stated on the record
  at FT-134; the depth law re-measured at
  both seams, at zero and four added wraps,
  over both oracle routes, with identity in
  its scoped form, at FT-135; the battery
  and the differential run in full at
  FT-136; the volume pass at a new seed with
  the golden fixture and the fleet pin
  executed at FT-137; and the gate at 247
  with its counts named, at FT-138. README's
  closed-rounds line moves from 22 of 100 to
  23 of 100 in this commit only. FT-83
  stands open and unrepaired exactly as
  FT-134 carried it, the Cargo.toml
  description variant assigned to a
  line-wide pass outside this round's scope,
  and is carried open by name into Round 24.
  The charges the Round 22 landing comment
  laid on this round stand discharged at
  FT-134, and nothing in this record claims
  a surface it did not run: the harnesses,
  the probes, the volume pass, and the gate
  each stand at the scope their items name.

- FT-140. Round 23's pull request 24 read
  back whole on its closed record through
  the GitHub API: zero reviews, zero review
  comments, and four issue comments, the
  shell 6850284721, the trigger 6850288280
  the owner posted, the auto-reply
  6850315396 (573 characters, already
  reviewed), and the landing comment
  6850656068 (1,807 characters). The shell
  stands in its true final form at this
  read at 11,023 characters, created
  2026-11-13T10:27:33Z and last edited
  2026-11-13T10:37:08Z, a completed review
  of the final head itself and not a skip:
  a walkthrough over FT-134 to FT-139;
  Merge Risk Minimal up to f7b95, the
  closure head
  f7b9571a28f470b2315b8cb22c9b5c4ee731b739,
  with the change assessment marked at
  that head, the commits reviewed 429f7048
  to f7b9571a, and the files selected for
  processing CONTRACT.md and README.md; the
  pre-merge checks at four passed, Title
  Check, Linked Issues Check, Out of Scope
  Changes Check, and Description Check,
  with none failed and none inconclusive;
  Docstring at zero occurrences; no
  rate-limit banner in the body; the line
  that no actionable comments were
  generated in the recent review standing
  in it; and Run ID
  b0a5f2ca-dafd-4a32-9c26-3df44b254d5e, with
  the included allowance used and zero
  remaining after it. The shell moved
  through two generations under that one
  comment ID: the pull request body's step
  0 account read it at 11,021 characters,
  last edited 2026-11-13T10:28:53Z, and the
  final edit of 2026-11-13T10:37:08Z stands
  at 11,023, the two-character drift the
  landing comment records. The body's
  account is bound to its read and is true
  at that moment on every figure it quotes;
  the final edit superseded figures only
  and no disposition fact: no actionable
  comment stands in either generation, and
  the four passed checks, the Minimal risk,
  and the absent Docstring section hold in
  both. No owner restatement stands in the
  body; the drift is recorded by the
  landing comment instead, so nothing in
  the Round 23 step 0 account is corrected
  here. Charge (b): FT-83 stands open and
  unrepaired, assigned outside this round's
  scope to a line-wide pass, and is carried
  open by name here. Charge (c): no late
  CodeRabbit finding stands on pull request
  24. Nothing was posted there after the
  landing comment, the shell's last edit
  predates the merge, and the auto-reply's
  already reviewed is the last word the bot
  left, so nothing is owed a disposition.

- FT-141. The depth law re-measured fresh on
  the binary built from Round 23's closing
  tree, at probes in FT-119's shape: tapes
  built by string construction, a single
  tool_call event whose input carries the
  payload, the counted unit the nested
  arrays wrapping the scalar 1 inside the
  input, and at W added wraps the arrays
  sitting inside W nested single-key
  objects inside the input. Every point
  stands found by binary search and stated
  as the last accepted input depth and the
  first refused. At zero added wraps the
  digest seam stands unmoved on all three
  arms: the binary digests at 997 and
  refuses at 998, the oracle by the fleet
  standard runner digests at 991 and
  refuses at 992, and the oracle by the
  module route digests at 989 and refuses
  at 990. At four added wraps every arm
  stands exactly four below its zero-wrap
  point, FT-135's measurements at that
  level reproduced: the binary at 993 and
  994, the standard runner at 987 and 988,
  the module route at 985 and 986. Every
  refusal on every arm wears the one form,
  exit 1 with the tape nested beyond the
  interpreter limit, so both of FT-119's
  laws hold at both levels on the closing
  tree: one array per added container level
  on every arm, and the oracle routes
  exactly two apart on the digest seam with
  the route named beside every
  interpreter-limit figure here. Identity
  holds in FT-126's scoped form over the
  same zero-wrap tapes: all three arms
  print the identical digest at 100, 500,
  900, 950, and 989, the printed digests
  compared byte for byte; the binary and
  the standard runner print the identical
  digest at 990 and 991, where the module
  route already refuses; and at 997 the
  binary prints its digest alone, past
  both oracle routes' limits. No all-three
  identity is claimed past 989 and no
  binary-and-standard identity past 991,
  the exact scope FT-126 set. At the
  validate seam the binary again stands
  apart from the interpreter: it validates
  with exit 0 at each of the eighteen
  named zero-wrap input depths, 990, 991,
  992, 997, 998, 999, 1000, 1001, 2000,
  5000, 9994, 9995, 10000, 20000, 50000,
  100000, 150000, and 200000, uncapped on
  this seam everywhere FT-121 looked and at
  the extremes again here. The oracle's
  JSON reader stops at its interpreter
  limit and its routes stand exactly three
  apart on this seam: at zero wraps the
  standard runner accepts 9,994 and
  refuses 9,995 and the module route
  accepts 9,991 and refuses 9,992,
  reproducing FT-121's points; at four
  wraps each route stands exactly four
  below its zero-wrap point, the standard
  runner accepting 9,990 and refusing
  9,991 and the module route accepting
  9,987 and refusing 9,988, FT-135's
  measurements reproduced, the routes
  three apart at both levels. Every oracle
  refusal on this seam wears the reader's
  form, exit 1 with a defect reporting the
  file is not valid JSON.

- FT-142. The FT-91 battery and the FT-56
  differential run in full against the
  binary built from Round 23's closing
  tree. The battery passes 54 of 54 cells
  over its two named drivers: the original
  driver's 40 cells over canonical key
  order and float rendering, and
  battery2.py's 14 cells over stability,
  determinism, and edge cases, the golden
  digest standing in battery2.py's
  convert-dir cell and the FT-77 posture
  cell holding its contracted shape. The
  total is stated from those named parts,
  40 and 14, and from no wider run. The
  differential runs its 117 cases with a
  single divergence at case 64, slice over
  deep600.json, exit 1 against 0 in the
  FT-54 band where the binary's own depth
  refusal is the contracted behaviour, the
  output divergence riding on that exit
  gap, and zero file divergences over the
  117 cases. Its corpus guards stand exact
  on the closing tree, counted in lines on
  this tree: tests/fixtures/
  timestamp_corpus.txt at 371 lines and
  tests/fixtures/number_sweep.txt at 6,284
  lines. The oracle both harnesses measured
  against stands verified at FT-144 through
  the channel named there.

- FT-143. The FT-97 volume pass runs at a
  new seed, 20261116, Round 23's having been
  20261115, the same shape the generator has
  held since Round 20 coming out of a fresh
  draw: the corpus measures from itself at
  21,035 lines, 7,736,621 bytes, and 27
  sessions, the deep payload reaching 122
  containers and the widest object holding
  500 keys, and the long-haul session
  carrying 7,500 events with run totals of
  750,000 input tokens, 135,000 output
  tokens, 748,500 cache-read tokens, 1,500
  tool calls, and 4,500 messages. All 19
  cells stand byte-identical over both
  implementations with no failures:
  convert, digest, verify, validate, play,
  rollup, self-compare, and slice across
  the long-haul session; digest over the
  window counts; the directory and tree
  conversions; the deep-and-wide tape's
  convert, digest, validate, and play; the
  dirty tape's convert and its two screen
  cells; and the long-haul screen. The
  long-haul digest is the corpus's own
  under this seed,
  bf8e119aa43249cbdb262401c68b146c296d2dc145df2bddbb0cfb8b1d5f06f3,
  and differs from Round 23's as the new
  draw requires. The golden
  fixture executes byte for byte:
  converting tests/fixtures/
  claude_session_synthetic.jsonl writes
  tapes of 1,663 bytes, byte-identical on
  the binary and on both oracle routes,
  and the binary and both oracle routes
  print digest
  cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3
  for it. The fleet pin
  re-confirms live through the API: the
  commit 74a83298aeb31c82e62b9fb0c613f5bc8600799e
  resolves, its tree
  afd4d7d13520411e003762548fc79082a1b9cf6e
  lists 58 entries untruncated, and exactly
  50 of them are blobs.

- FT-144. The gate run against Round 23's
  closing tree. Cargo test passes 247 tests
  with zero failures, the count stated
  target by target: lib 7, cli 22, compare
  19, convert 34, digest_cli 5, identity 13,
  json 8, play 17, redaction 12, robustness
  15, rollup 22, scale 3, schema 5, screen
  15, slice 24, timestamps 1, validate 25,
  and doc 0, the counts summing to 247 and
  the doc count a true zero. Clippy over
  all targets with warnings denied exits 0,
  and cargo fmt in check mode exits 0. The
  codepoint scan stands at zero on both
  instruments: the repository's own
  scripts/codepoint_scan.py over the tree
  reports U+2014 at zero and U+2013 at zero,
  and this round's own Python scan over all
  52 blobs of the final tree reports the
  same two zeros, each counted by
  codepoint. The oracle the harnesses and
  volume pass measured against stands
  verified blob for blob against commit
  eca7b60dac2114ff026f8856fd19de05867d7304
  through the local clone's object store
  at ~/workspace/retro-line/flighttape: all
  11 files under that commit's
  src/flighttape match the working copy at
  ~/workspace/retro-line/ft15-scratch/
  oracle/src/flighttape blob for blob, with
  zero mismatches. The local clone's object
  store is the channel this verification
  used, stated exactly: that commit is not
  reachable through the GitHub API in this
  repository, as FT-133 records, and no
  part of this item claims the API channel
  for it.

- FT-145. Round 24 closes. Its record stands
  at FT-140 to FT-145: the Round 23
  read-back, with the shell contracted in
  its true final form, at FT-140; the depth
  law re-measured at both seams, at zero
  and four added wraps, over both oracle
  routes, with identity in its scoped form,
  at FT-141; the battery and the
  differential run in full at FT-142; the
  volume pass at a new seed with the golden
  fixture and the fleet pin executed at
  FT-143; and the gate at 247 with its
  counts named, at FT-144. README's
  closed-rounds line moves from 23 of 100
  to 24 of 100 in this commit only. FT-83
  stands open and unrepaired exactly as
  FT-140 carried it, the Cargo.toml
  description variant assigned to a
  line-wide pass outside this round's
  scope, and is carried open by name into
  Round 25. The charges the Round 23
  landing comment laid on this round stand
  discharged at FT-140, and nothing in this
  record claims a surface it did not run:
  the harnesses, the probes, the volume
  pass, and the gate each stand at the
  scope their items name.

- FT-146. Repair record, appended after
  the gate and before the merge. The spec
  fidelity review and the red team
  re-audit each returned the same Major
  against this round, and this item
  repairs it. PR 25's step 0 disposition
  dismissed the one actionable finding of
  the step 0 review (review 5898574244,
  review comment 4569074982, that FT-143's
  two digests stand split across this
  file's wrap and should stand whole on
  one line) on grounds the record does
  not bear. The census at the reviewed
  head
  3ba9de4f9b7993d2421ee7a0900f8e57ca115b5d
  stands: fifteen digests of 64 hex
  characters stand whole on single lines
  in this file and exactly two stand
  split, both of them FT-143's, and
  FT-137's long-haul and golden digests
  stand whole on single lines at the
  base. The finding was valid. It is
  repaired here, not dismissed: FT-143's
  long-haul digest and golden digest now
  stand whole on single lines in FT-137's
  standing form, their values unchanged,
  and the repair moves the two digests to
  the whole side of the census, so no
  digest in this file now stands split.
  The dismissal paragraph in PR 25's body
  stands corrected by an owner repair
  note of the same date as this item.
  Apart from FT-143's two digest lines
  and the lines their joining reflows,
  everything in FT-140 to FT-145 stands
  as reviewed, byte for byte.

- FT-147. PR 25 read back whole on its closed
  record, read through the API on 2026-11-14:
  4 issue comments, 1 review, and 1 review
  comment, every body read to its end, and the
  body read to its end including both owner
  notes appended to it. The issue comments are
  the bot's shell 6865820516, the owner's
  trigger 6865822134 at 20 characters, the
  bot's auto-reply 6865899912 at 573
  characters, its standing text that the last
  commit already stands reviewed, and the
  owner's landing comment 6866938272 at 2,867
  characters. The shell in its true final form
  at this read stands at 8,368 characters by a
  Python count over the API body, created
  2026-11-13T21:52:13Z and last edited
  2026-11-13T22:50:37Z. The earlier figures
  stand at their own reads and are not
  falsified: 8,360 characters at the body's
  bound read, the last edit then at
  2026-11-13T21:54:12Z, and 8,362 at the
  reviews' reads, the last edit then at
  2026-11-13T22:02:44Z. The shell's final edit
  at 22:50:37Z postdates the repair head and
  precedes the landing comment, and no
  disposition fact moved in it. In its final
  form the shell carries a walkthrough over
  FT-140 to FT-145; its change assessment
  pinned to the reviewed head
  3ba9de4f9b7993d2421ee7a0900f8e57ca115b5d as
  both the source and the covered commit, kind
  reviewed; Merge Risk Low up to 3ba9d;
  pre-merge checks at 4 passed, Title, Linked
  Issues, Out of Scope Changes, and
  Description; no rate-limit banner; and no
  Run ID line and no Docstring section
  standing in it. Its merge risk prose itself
  notes the two wrapped digests. The one
  review, 5898574244, stands COMMENTED on the
  reviewed head, its Run ID
  8a674212-6db0-43f4-b9ca-d77adc695ac7
  standing in the review body, with one
  actionable comment, review comment
  4569074982, a Minor asking that FT-143's two
  digests stand whole on one line, and the
  comment carries the bot's own later marker
  that it stands addressed in commit 7c9efb7,
  from the comment's edit at
  2026-11-13T22:40:49Z; no replies stand to
  it. Nothing stands on PR 25 after the
  landing comment: no comment, no review, and
  no edit of any kind postdates
  2026-11-13T22:52:59Z, so the late findings
  charge discharges as empty. The Round 24
  gate, graded from FT-146 and the body
  together as charged: FT-146, read whole,
  records the census at the reviewed head,
  fifteen digests of 64 hex characters
  standing whole on single lines and exactly
  two standing split, both of them FT-143's,
  and this read-back re-counts that census at
  the three trees and finds it exact. At the
  reviewed head the two splits stand at
  FT-143's own lines, the long-haul digest
  split across lines 3568 and 3569 and the
  golden digest across lines 3578 and 3579 of
  that file; at the base FT-137's long-haul
  and golden digests stand whole on single
  lines; and on main as merged both of
  FT-143's digests stand whole on single lines
  with their values unchanged, and no digest
  in the file stands split. The body's owner
  repair note states the dismissal does not
  stand and the finding was valid, the body's
  Known deviations sentence stands scoped to
  PR 24's record by its owner note, and the
  delta reviews of the repair head stand clean
  on both axes as the body and the landing
  comment record them. The record as it stands
  is coherent, and the standing rule the round
  earned, that a dismissal in this line cites
  a census it has counted or it does not
  stand, is the form this read-back itself
  follows. FT-83 stands carried open by name,
  untouched.

- FT-148. The depth law re-measured fresh on
  the binary built from Round 24's closing
  tree, at probes in FT-119's shape: tapes
  built by string construction, a single
  tool_call event whose input carries the
  payload, the counted unit the nested arrays
  wrapping the scalar 1 inside the input, and
  at W added wraps the arrays sitting inside W
  nested single-key objects inside the input;
  the event carries the fields the validator
  requires, sidechain, call_id, and tool, as
  siblings of the input, standing off the
  counted path. Every point stands found by
  binary search and stated as the last
  accepted input depth and the first refused.
  At zero added wraps the digest seam stands
  unmoved on all three arms: the binary
  digests at 997 and refuses at 998, the
  oracle by the fleet standard runner digests
  at 991 and refuses at 992, and the oracle by
  the module route digests at 989 and refuses
  at 990. At four added wraps every arm stands
  exactly four below its zero-wrap point,
  FT-135's measurements at that level
  reproduced: the binary at 993 and 994, the
  standard runner at 987 and 988, the module
  route at 985 and 986. Every refusal on every
  arm wears the one form, exit 1 with the tape
  nested beyond the interpreter limit, so both
  of FT-119's laws hold at both levels on the
  closing tree: one array per added container
  level on every arm, and the oracle routes
  exactly two apart on the digest seam with
  the route named beside every
  interpreter-limit figure here. Identity
  holds in FT-126's scoped form over the same
  zero-wrap tapes: all three arms print the
  identical digest at 100, 500, 900, 950, and
  989, the printed digests compared byte for
  byte; the binary and the standard runner
  print the identical digest at 990 and 991,
  where the module route already refuses; and
  at 997 the binary prints its digest alone,
  past both oracle routes' limits. No
  all-three identity is claimed past 989 and
  no binary-and-standard identity past 991,
  the exact scope FT-126 set. At the validate
  seam the binary again stands apart from the
  interpreter: it validates with exit 0 at
  each of the eighteen named zero-wrap input
  depths, 990, 991, 992, 997, 998, 999, 1000,
  1001, 2000, 5000, 9994, 9995, 10000, 20000,
  50000, 100000, 150000, and 200000, uncapped
  on this seam everywhere FT-121 looked and at
  the extremes again here. The oracle's JSON
  reader stops at its interpreter limit and
  its routes stand exactly three apart on this
  seam: at zero wraps the standard runner
  accepts 9,994 and refuses 9,995 and the
  module route accepts 9,991 and refuses
  9,992, reproducing FT-121's points; at four
  wraps each route stands exactly four below
  its zero-wrap point, the standard runner
  accepting 9,990 and refusing 9,991 and the
  module route accepting 9,987 and refusing
  9,988, FT-135's measurements reproduced, the
  routes three apart at both levels. Every
  oracle refusal on this seam wears the
  reader's form, exit 1 with a defect
  reporting the file is not valid JSON.

- FT-149. The FT-91 battery and the FT-56
  differential harness run fresh at the same
  binary. The battery stands at 54 of 54, run
  by its two named drivers: battery.py at 40
  cells, 12 key and 28 float, and battery2.py
  at 14 stability cells S0 to S13, S13 holding
  the FT-77 whole-argv non-UTF8 posture; no
  cell failed in either driver. The
  differential runs its 117 cases with the
  single divergence at case 64, slice over the
  deep tape, the binary at exit 1 against the
  oracle's exit 0 in the FT-54 band, exactly
  as contracted, and zero file divergences
  stand anywhere in the run. The corpus guards
  stand exact on this tree:
  tests/fixtures/timestamp_corpus.txt at 371
  lines and tests/fixtures/number_sweep.txt at
  6,284 lines, both counted on this tree at
  lane time.

- FT-150. The volume pass under a new seed,
  20261121: the corpus measures from itself at
  21,035 lines and 7,736,621 bytes over 27
  sessions, its long-haul tape at 7,500 events
  with totals 750,000 input, 135,000 output,
  and 748,500 cache-read tokens, 1,500 tool
  calls, and 4,500 messages. All 19 cells
  stand byte-identical between the binary and
  the oracle, stdout, stderr, exit codes, and
  every written file compared, and the
  long-haul digest under this seed is
  7037012bafa31869bf3ea0e546d9ef1fb8b63034c22b59f16da930c23e4ee35a,
  differing from Round 24's as the new draw
  requires. The golden fixture executes byte
  for byte: converting
  tests/fixtures/claude_session_synthetic.jsonl
  writes tapes of 1,663 bytes, byte-identical
  on the binary and on both oracle routes, and
  the binary and both oracle routes print
  digest
  cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3
  for it. The fleet pin re-confirms live
  through the API: the commit
  74a83298aeb31c82e62b9fb0c613f5bc8600799e
  resolves, its tree
  afd4d7d13520411e003762548fc79082a1b9cf6e
  lists 58 entries untruncated, and exactly 50
  of them are blobs.

- FT-151. The gate run against Round 24's
  closing tree. Cargo test passes 247 tests
  with zero failures, the count stated target
  by target: lib 7, cli 22, compare 19,
  convert 34, digest_cli 5, identity 13, json
  8, play 17, redaction 12, robustness 15,
  rollup 22, scale 3, schema 5, screen 15,
  slice 24, timestamps 1, validate 25, and doc
  0, the counts summing to 247 and the doc
  count a true zero. Clippy over all targets
  with warnings denied exits 0, and cargo fmt
  in check mode exits 0. The codepoint scan
  stands at zero on both instruments: the
  repository's own scripts/codepoint_scan.py
  over the tree reports U+2014 = 0 and U+2013
  = 0, and this round's own scan over all 52
  blobs of the final tree reports the same.
  The oracle stands verified blob for blob
  against commit
  eca7b60dac2114ff026f8856fd19de05867d7304
  through the local clone's object store: the
  eleven files under its src/flighttape match
  the working copy at the driven extraction
  with zero mismatches. That commit is not
  reachable through the GitHub API in this
  repository, as FT-133 records; the channel
  is stated exactly and no part of this item
  claims the API channel for it.

- FT-152. Round 25 closes. Its record stands
  at FT-147 to FT-152: the Round 24 read-back,
  with the shell contracted in its true final
  form and the gate graded from FT-146 and the
  body together, at FT-147; the depth law
  re-measured at both seams, at zero and four
  added wraps, over both oracle routes, with
  identity in its scoped form, at FT-148; the
  battery and the differential run in full at
  FT-149; the volume pass at a new seed with
  the golden fixture and the fleet pin
  executed at FT-150; and the gate at 247 with
  its counts named, at FT-151. README's
  closed-rounds line moves from 24 of 100 to
  25 of 100 in this commit only. FT-83 stands
  open and unrepaired exactly as FT-147
  carried it, the Cargo.toml description
  variant assigned to a line-wide pass outside
  this round's scope, and is carried open by
  name into Round 26. The charges the Round 24
  landing comment laid on this round stand
  discharged at FT-147, and nothing in this
  record claims a surface it did not run: the
  harnesses, the probes, the volume pass, and
  the gate each stand at the scope their items
  name.

- FT-153. PR 26 read back whole on its closed
  record, read through the API on 2026-11-14:
  4 issue comments, no reviews, and no review
  comments, every body read to its end. The
  issue comments are the owner's trigger
  6873725288 at 20 characters, the bot's shell
  6873726978, the bot's auto-reply 6873754363
  at 573 characters, its standing text that
  the last commit already stands reviewed, and
  the owner's landing comment 6873933258 at
  1,861 characters. The shell in its true
  final form at this read stands at 10,795
  characters by a Python count over the API
  body, created 2026-11-14T07:20:49Z and last
  edited 2026-11-14T07:26:46Z. The earlier
  figure stands at its own read and is not
  falsified: 10,793 characters at the body's
  bound read, and the +2 edit at 07:26:46Z
  that brought the shell to its final form
  precedes the landing comment at 07:37:38Z,
  no disposition fact moving in it. In its
  final form the shell carries a walkthrough
  over FT-147 to FT-152 as Round 25
  documentation; its change assessment pinned
  to the closure head
  0eee81b02069bfdf61b98d72f5a18afd255605ee as
  both the source and the covered commit, kind
  reviewed, over the two files CONTRACT.md and
  README.md, the reviewed range running from
  the base
  0e39907425475227b6f282fed708bc3ffd856cf1 to
  that head; Merge Risk Minimal up to 0eee8;
  pre-merge checks at 4 passed, Title, Linked
  Issues, Out of Scope Changes, and
  Description, the Title check's full
  explanation carrying only the remark that
  the title does not say the change is
  documentation-only, prose on a passed check
  and no finding; no Docstring section
  standing in the shell; the shell's own line
  that no actionable comments were generated
  in the recent review; its Run ID
  2f811c0a-cda5-4053-86e8-4c44a0f7f6f0
  standing in the shell itself; and no
  rate-limit banner. Nothing stands on PR 26
  after the landing comment: no comment, no
  review, and no edit of any kind postdates
  2026-11-14T07:37:38Z, so the late findings
  charge discharges as empty. The Round 25
  gate stands exactly as the landing comment
  records it: spec fidelity PASS, standards
  APPROVE, and red team APPROVE, no findings
  on any axis, hosted CI green on the exact
  head at runs 43570524110 and 43570650407,
  and step 0 with nothing actionable and
  nothing dismissed anywhere in that round's
  record. The record as it stands is coherent.
  FT-83 stands carried open by name,
  untouched: Cargo.toml's description stands
  as the third variant, "Flight tapes for
  agent runs: validate, convert, play back,
  compare, and screen recorded sessions.",
  against the canonical line the repository
  description and the README lede share word
  for word, and its alignment stands assigned
  to the line-wide pass outside this round's
  scope.

- FT-154. The depth law re-measured fresh on
  the binary built from Round 25's closing
  tree, at probes in FT-119's shape: tapes
  built by string construction, a single
  tool_call event whose input carries the
  payload, the counted unit the nested arrays
  wrapping the scalar 1 inside the input, and
  at W added wraps the arrays sitting inside W
  nested single-key objects inside the input;
  the event carries the fields the validator
  requires, sidechain, call_id, and tool, as
  siblings of the input, standing off the
  counted path. Every point stands found by
  binary search and stated as the last
  accepted input depth and the first refused.
  At zero added wraps the digest seam stands
  unmoved on all three arms: the binary
  digests at 997 and refuses at 998, the
  oracle by the fleet standard runner digests
  at 991 and refuses at 992, and the oracle by
  the module route digests at 989 and refuses
  at 990. At four added wraps every arm stands
  exactly four below its zero-wrap point,
  FT-135's measurements at that level
  reproduced: the binary at 993 and 994, the
  standard runner at 987 and 988, the module
  route at 985 and 986. Every refusal on every
  arm wears the one form, exit 1 with the tape
  nested beyond the interpreter limit, so both
  of FT-119's laws hold at both levels on the
  closing tree: one array per added container
  level on every arm, and the oracle routes
  exactly two apart on the digest seam with
  the route named beside every
  interpreter-limit figure here. Identity
  holds in FT-126's scoped form over the same
  zero-wrap tapes: all three arms print the
  identical digest at 100, 500, 900, 950, and
  989, the printed digests compared byte for
  byte; the binary and the standard runner
  print the identical digest at 990 and 991,
  where the module route already refuses; and
  at 997 the binary prints its digest alone,
  past both oracle routes' limits. No
  all-three identity is claimed past 989 and
  no binary-and-standard identity past 991,
  the exact scope FT-126 set. At the validate
  seam the binary again stands apart from the
  interpreter: it validates with exit 0 at
  each of the eighteen named zero-wrap input
  depths, 990, 991, 992, 997, 998, 999, 1000,
  1001, 2000, 5000, 9994, 9995, 10000, 20000,
  50000, 100000, 150000, and 200000, uncapped
  on this seam everywhere FT-121 looked and at
  the extremes again here. The oracle's JSON
  reader stops at its interpreter limit and
  its routes stand exactly three apart on this
  seam: at zero wraps the standard runner
  accepts 9,994 and refuses 9,995 and the
  module route accepts 9,991 and refuses
  9,992, reproducing FT-121's points; at four
  wraps each route stands exactly four below
  its zero-wrap point, the standard runner
  accepting 9,990 and refusing 9,991 and the
  module route accepting 9,987 and refusing
  9,988, FT-135's measurements reproduced, the
  routes three apart at both levels. Every
  oracle refusal on this seam wears the
  reader's form, exit 1 with a defect
  reporting the file is not valid JSON.
