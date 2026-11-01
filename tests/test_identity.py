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
    # Whole numbers, the point at or past the last digit.
    (0.0, "0"), (-0.0, "0"), (2.0, "2"), (100.0, "100"),
    (1e16, "10000000000000000"), (1e20, "100000000000000000000"),
    (9007199254740991.0, "9007199254740991"),
    (4.35e15, "4350000000000000"),
    # The point inside the digits.
    (0.95, "0.95"), (-2.5, "-2.5"), (0.1, "0.1"),
    (123456789.123, "123456789.123"), (1234567.891, "1234567.891"),
    (1 / 3, "0.3333333333333333"),
    # Small fractions, the point before the first digit.
    (0.000123, "0.000123"), (1e-5, "0.00001"), (2.5e-5, "0.000025"),
    (1e-6, "0.000001"),
    # Exponential form.
    (1e-7, "1e-7"), (1.5e-7, "1.5e-7"), (1e21, "1e+21"),
    (1.5e21, "1.5e+21"), (7e22, "7e+22"), (5e-324, "5e-324"),
    (1.7976931348623157e308, "1.7976931348623157e+308"),
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


def test_key_order_and_whitespace_do_not_change_the_digest():
    tape = fixture_tape()
    reloaded = json.loads(json.dumps(tape, sort_keys=True, indent=7))
    assert tape_digest(reloaded) == GOLDEN
    assert canonical_bytes(reloaded) == canonical_bytes(tape)


def test_keys_sort_by_utf16_code_units_as_the_scheme_does():
    tape = {"": 1, "\U00010000": 2}
    assert canonical_bytes(tape).decode("utf-8") == (
        '{"\U00010000":2,"":1}')


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
    tape["reading"] = math.nan
    defects = validate_tape(tape)
    assert any("non-finite" in defect and "reading" in defect
               for defect in defects)
    with pytest.raises(DigestError):
        canonical_bytes(tape)


def test_a_shared_substructure_is_named_at_every_location():
    shared = {"reading": math.nan}
    defects = validate_tape({"a": shared, "b": shared})
    named = [d for d in defects if "non-finite" in d]
    assert any("a.reading" in d for d in named)
    assert any("b.reading" in d for d in named)


def test_a_non_finite_dict_key_is_named():
    defects = validate_tape({"run": {math.inf: 1}})
    assert any("non-finite" in d and "as a key, inf" in d
               for d in defects)


def test_two_non_finite_keys_earn_two_distinct_defects():
    defects = validate_tape({"run": {math.inf: 1, -math.inf: 2}})
    named = [d for d in defects if "as a key" in d]
    assert len(named) == 2 and named[0] != named[1]


def test_walk_defects_arrive_in_document_order():
    defects = validate_tape({"z": math.nan, "a": math.nan})
    named = [d for d in defects if "non-finite" in d]
    assert named == ["z is a non-finite number",
                     "a is a non-finite number"]


def test_unpaired_surrogates_are_refused_by_name():
    with pytest.raises(DigestError, match="cannot be encoded"):
        canonical_bytes({"text": "\ud800"})
    with pytest.raises(DigestError, match="cannot be encoded"):
        canonical_bytes({"\ud800": 1})


def test_an_integer_beyond_the_rendering_limit_is_refused():
    with pytest.raises(DigestError, match="cannot be rendered"):
        canonical_bytes(10 ** 5000)


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
    assert tape_digest({"not": "a tape"}) == (
        "ab587bf78c80e183317a77423a19a8f3b2633d98b1b3ed95b63d315b4d6f3c07")


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


def test_cli_digest_of_a_surrogate_tape_is_a_named_error(
        tmp_path, capsys):
    path = tmp_path / "tape.json"
    path.write_text('{"text": "\\ud800"}', encoding="utf-8")
    assert main(["digest", str(path)]) == 1
    captured = capsys.readouterr()
    assert captured.out == ""
    assert "cannot be digested" in captured.err


def test_a_null_tape_file_is_named_not_silent(tmp_path, capsys):
    path = tmp_path / "tape.json"
    path.write_text("null\n", encoding="utf-8")
    assert main(["validate", str(path)]) == 1
    assert "not a JSON object" in capsys.readouterr().err
    assert main(["play", str(path)]) == 1
    assert "not a JSON object" in capsys.readouterr().err
    null_digest = ("74234e98afe7498fb5daf1f36ac2d78acc339464"
                   "f950703b8c019892f982b90b")
    assert main(["digest", str(path)]) == 0
    assert capsys.readouterr().out == null_digest + "\n"
    assert main(["verify", str(path), null_digest]) == 0
    capsys.readouterr()
