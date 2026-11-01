"""Tests for tape identity, FT-26 to FT-30.

Every test can fail for a real reason. The number table fails if
the rendering drifts from the ECMAScript algorithm, the golden
digest fails if the canonical form changes in any byte, the
mutation table fails if some part of the tape escapes the
canonical form, and the refusal tests fail if a non-finite
number, a non-string key, or a cyclic or over-deep structure
raises or hangs instead of being named.
"""
import copy
import json
import math
import pathlib

import pytest

from flighttape.cli import main
from flighttape.convert_claude import convert_lines
from flighttape.identity import DigestError, canonical_bytes, tape_digest
from flighttape.validate import validate_tape

FIXTURE = (pathlib.Path(__file__).parent / "fixtures"
           / "claude_session_synthetic.jsonl")
GOLDEN = "cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3"


def fixture_tape():
    return convert_lines(FIXTURE.read_text(encoding="utf-8").splitlines())


NUMBER_TABLE = [
    (0.0, "0"), (-0.0, "0"), (2.0, "2"), (-2.5, "-2.5"), (0.95, "0.95"),
    (1e-5, "0.00001"), (1e-6, "0.000001"), (1e-7, "1e-7"),
    (1.5e-7, "1.5e-7"), (1e16, "10000000000000000"),
    (1e20, "100000000000000000000"), (1e21, "1e+21"), (1.5e21, "1.5e+21"),
    (123456789.123, "123456789.123"), (5e-324, "5e-324"),
    (1.7976931348623157e308, "1.7976931348623157e+308"), (100.0, "100"),
    (0.1, "0.1"), (0.000123, "0.000123"), (1234567.891, "1234567.891"),
    (9007199254740991.0, "9007199254740991"), (1 / 3, "0.3333333333333333"),
    (2.5e-5, "0.000025"), (7e22, "7e+22"), (4.35e15, "4350000000000000"),
]


@pytest.mark.parametrize("value, rendered", NUMBER_TABLE)
def test_numbers_render_as_ecmascript_renders_them(value, rendered):
    assert canonical_bytes(value).decode("utf-8") == rendered
    assert float(rendered) == value


def test_canonical_bytes_are_exact_for_a_small_tape():
    tape = {"b": [1, 2.0, "café"], "a": {"z": None, "m": True}}
    assert canonical_bytes(tape).decode("utf-8") == (
        '{"a":{"m":true,"z":null},"b":[1,2,"café"]}')


def test_the_fixture_tape_digests_to_the_pinned_golden():
    assert tape_digest(fixture_tape()) == GOLDEN


def test_key_order_and_whitespace_do_not_change_the_digest(tmp_path):
    tape = fixture_tape()
    shuffled = json.loads(json.dumps(tape, sort_keys=True))
    path = tmp_path / "tape.json"
    path.write_text(json.dumps(shuffled, indent=7), encoding="utf-8")
    reloaded = json.loads(path.read_text(encoding="utf-8"))
    assert tape_digest(reloaded) == GOLDEN
    assert canonical_bytes(reloaded) == canonical_bytes(tape)


def test_every_part_of_the_tape_moves_the_digest():
    tape = fixture_tape()
    mutations = []
    changed = copy.deepcopy(tape)
    changed["run"]["id"] = "another-session"
    mutations.append(changed)
    changed = copy.deepcopy(tape)
    changed["events"][1]["text"] = "changed wording"
    mutations.append(changed)
    changed = copy.deepcopy(tape)
    changed["run"]["totals"]["tool_calls"] = 99
    mutations.append(changed)
    changed = copy.deepcopy(tape)
    changed["outcome"]["status"] = "failed"
    mutations.append(changed)
    changed = copy.deepcopy(tape)
    changed["events"] = changed["events"][:-1]
    mutations.append(changed)
    for mutation in mutations:
        assert tape_digest(mutation) != GOLDEN


def test_a_non_finite_number_is_refused_and_named_by_the_validator():
    tape = fixture_tape()
    tape["events"][2]["input"]["score"] = math.nan
    with pytest.raises(DigestError):
        tape_digest(tape)
    defects = validate_tape(tape)
    assert any("non-finite" in defect and "events[2]" in defect
               for defect in defects)


def test_an_infinity_in_a_run_field_is_named():
    tape = fixture_tape()
    tape["run"]["totals"]["cost"] = math.inf
    defects = validate_tape(tape)
    assert any("non-finite" in defect and "totals" in defect
               for defect in defects)
    with pytest.raises(DigestError):
        canonical_bytes(tape)


def test_a_cyclic_tape_is_named_not_hung():
    tape = {"run": {}}
    tape["run"]["self"] = tape
    defects = validate_tape(tape)
    assert isinstance(defects, list) and defects
    with pytest.raises(DigestError):
        canonical_bytes(tape)


def test_a_deeply_nested_tape_is_named_not_crashed():
    deep = current = {}
    for _ in range(5000):
        current["x"] = {}
        current = current["x"]
    current["leaf"] = math.nan
    defects = validate_tape({"wrap": deep})
    assert any("non-finite" in defect for defect in defects)
    with pytest.raises(DigestError):
        canonical_bytes(deep)


def test_the_walk_runs_even_on_a_structural_early_return():
    tape = {"run": {"cost": math.inf}, "events": "not a list"}
    defects = validate_tape(tape)
    assert any("non-finite" in defect for defect in defects)


def test_a_non_string_object_key_is_refused():
    with pytest.raises(DigestError):
        canonical_bytes({1: "a"})


def test_digest_identifies_content_not_validity():
    assert len(tape_digest({"not": "a tape"})) == 64


def test_cli_digest_prints_exactly_the_digest(tmp_path, capsys):
    path = tmp_path / "tape.json"
    path.write_text(json.dumps(fixture_tape()), encoding="utf-8")
    assert main(["digest", str(path)]) == 0
    assert capsys.readouterr().out == GOLDEN + "\n"


def test_cli_verify_accepts_and_rejects(tmp_path, capsys):
    path = tmp_path / "tape.json"
    path.write_text(json.dumps(fixture_tape()), encoding="utf-8")
    assert main(["verify", str(path), GOLDEN]) == 0
    assert "digest matches" in capsys.readouterr().out
    assert main(["verify", str(path), "  " + GOLDEN.upper()]) == 0
    capsys.readouterr()
    assert main(["verify", str(path), "0" * 64]) == 1
    assert "digest mismatch" in capsys.readouterr().err


def test_cli_digest_of_an_undigestible_tape_is_a_named_error(
        tmp_path, capsys):
    path = tmp_path / "tape.json"
    path.write_text('{"events": [{"input": {"x": NaN}}]}',
                    encoding="utf-8")
    assert main(["digest", str(path)]) == 1
    captured = capsys.readouterr()
    assert captured.out == ""
    assert "cannot be digested" in captured.err
    assert main(["validate", str(path)]) == 1
    assert "non-finite" in capsys.readouterr().err
