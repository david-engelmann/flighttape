"""Robustness tests (FT-18 to FT-21).

The converter and the validator sit on the trust boundary of the
tape format, so this suite feeds them a deterministic corpus of
mutated fixture lines plus a fixed table of adversarial tapes and
holds one property throughout, nothing raises. The corpus is
generated from a fixed seed and its size is pinned, so a change
to the generator is a deliberate act. Every fixture line and
every adversarial value here is synthetic.
"""
import copy, json, pathlib, random

from flighttape.cli import main
from flighttape.convert_claude import convert_file, convert_lines
from flighttape.validate import validate_tape

FIXTURE = (pathlib.Path(__file__).resolve().parent / "fixtures"
           / "claude_session_synthetic.jsonl")
SEED = 20261101
CASES = 240

SUBSTITUTIONS = [
    ('"user"', '"assistant"'),
    ('"assistant"', '"user"'),
    ('"text"', '"tool_use"'),
    ('"tool_use"', '"tool_result"'),
    ('"Read"', '"Read'),
    ("150", "9" * 6000),
    ("true", "tru"),
    ('"system"', '"user"'),
]
FLIP_CHARS = '{}[]",:0123456789truefalsn\\'


def fixture_lines():
    return [line for line in
            FIXTURE.read_text(encoding="utf-8").splitlines() if line.strip()]


def corpus():
    rng = random.Random(SEED)
    lines = fixture_lines()
    cases = []
    for _ in range(CASES):
        line = rng.choice(lines)
        move = rng.randrange(5)
        if move == 0:
            line = line[:rng.randrange(len(line) + 1)]
        elif move == 1:
            at = rng.randrange(len(line))
            line = line[:at] + rng.choice(FLIP_CHARS) + line[at + 1:]
        elif move == 2:
            old, new = rng.choice(SUBSTITUTIONS)
            line = line.replace(old, new, 1)
        elif move == 3:
            line = line + line[:rng.randrange(len(line) + 1)]
        else:
            line = line.replace("{", "", 1).replace("}", "", 1)
        cases.append(line)
    return cases


def test_corpus_is_deterministic_and_pinned():
    assert len(corpus()) == CASES
    assert corpus() == corpus()


def test_corpus_never_raises_and_every_result_validates():
    for line in corpus():
        tape = convert_lines([line])
        assert isinstance(tape["events"], list)
        defects = validate_tape(tape)
        assert isinstance(defects, list)


def test_undecodable_lines_are_counted_not_fatal(tmp_path):
    good = fixture_lines()[0]
    path = tmp_path / "mixed.jsonl"
    path.write_bytes(good.encode("utf-8") + b"\n\xff\xfe not utf-8 \x80\n")
    tape = convert_file(path)
    assert tape["run"]["skipped_lines"].get("undecodable") == 1
    assert len(tape["events"]) >= 1


def test_huge_integer_line_is_counted_unparseable():
    line = '{"type":"user","message":{"content":' + "9" * 6000 + "}}"
    tape = convert_lines([line])
    assert tape["events"] == []
    assert tape["run"]["skipped_lines"] == {"unparseable": 1}


def test_deeply_nested_tool_input_is_counted_whole():
    nested = json.loads('{"a":' * 3000 + "1" + "}" * 3000)
    line = json.dumps({
        "type": "assistant", "sessionId": "synthetic-deep",
        "message": {"role": "assistant", "content": [
            {"type": "tool_use", "id": "c1", "name": "Read",
             "input": nested}]}})
    tape = convert_lines([line])
    assert tape["events"] == []
    assert tape["run"]["skipped_lines"] == {"too-deep": 1}


def good_tape():
    return {
        "tape_version": "1.0",
        "run": {"harness": "claude-code", "session_id": "s1"},
        "events": [
            {"seq": 0, "kind": "tool_call", "sidechain": False,
             "call_id": "c1", "tool": "Read"},
            {"seq": 1, "kind": "tool_result", "sidechain": False,
             "call_id": "c1"},
        ],
        "outcome": {"status": "completed"},
    }


def adversarial_tapes():
    cases = {}

    def case(name, mutate):
        tape = copy.deepcopy(good_tape())
        mutate(tape)
        cases[name] = tape

    case("kind as list", lambda t: t["events"][0].update(kind=["tool_call"]))
    case("kind as dict", lambda t: t["events"][0].update(kind={"k": 1}))
    case("status as list", lambda t: t["outcome"].update(status=["completed"]))
    case("status as dict with basis",
         lambda t: t["outcome"].update(status={"s": 1}, basis="no-events"))
    case("basis as dict", lambda t: t["outcome"].update(basis={"b": 1}))
    case("result call_id as list",
         lambda t: t["events"][1].update(call_id=["c1"]))
    case("call call_id as dict",
         lambda t: t["events"][0].update(call_id={"c": 1}))
    case("run as list", lambda t: t.update(run=["x"]))
    case("events as dict", lambda t: t.update(events={"e": 1}))
    case("event as string", lambda t: t["events"].append("not an event"))
    case("timestamp as list",
         lambda t: t["events"][0].update(timestamp=["2026-01-01"]))
    case("totals value as dict",
         lambda t: t["run"].update(totals={"tool_calls": {"n": 1}}))
    case("version as list", lambda t: t.update(tape_version=["1.0"]))
    case("sidechain as string",
         lambda t: t["events"][0].update(sidechain="no"))
    return cases


def test_adversarial_tapes_are_named_defects_never_exceptions():
    for name, tape in adversarial_tapes().items():
        defects = validate_tape(tape)
        assert isinstance(defects, list) and defects, name


def test_cli_convert_missing_source_is_a_named_error(tmp_path, capsys):
    missing = tmp_path / "nope.jsonl"
    assert main(["convert", str(missing)]) == 1
    assert "no such source file" in capsys.readouterr().err
    assert main(["convert-dir", str(missing), "-o",
                 str(tmp_path / "out")]) == 1
    assert "no such source" in capsys.readouterr().err


def test_cli_validate_and_play_reject_an_undecodable_file(tmp_path, capsys):
    bad = tmp_path / "bad.json"
    bad.write_bytes(b'{"tape_version": "1.0", \xff\xfe}')
    assert main(["validate", str(bad)]) == 1
    assert "UTF-8" in capsys.readouterr().err
    assert main(["play", str(bad)]) == 1
    assert "UTF-8" in capsys.readouterr().err


def test_cli_validate_a_missing_tape_is_a_named_error(tmp_path, capsys):
    assert main(["validate", str(tmp_path / "nope.json")]) == 1
    assert "cannot read the tape file" in capsys.readouterr().err


def test_cli_play_rejects_a_tape_with_an_unhashable_kind(tmp_path, capsys):
    tape = good_tape()
    tape["events"][0]["kind"] = ["tool_call"]
    path = tmp_path / "tape.json"
    path.write_text(json.dumps(tape), encoding="utf-8")
    assert main(["validate", str(path)]) == 1
    assert "kind" in capsys.readouterr().err
    assert main(["play", str(path)]) == 1
    assert "kind" in capsys.readouterr().err
