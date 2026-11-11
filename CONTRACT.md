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
