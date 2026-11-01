from flighttape.validate import validate_tape


def good_tape():
    return {
        "tape_version": "1.0",
        "run": {"harness": "claude-code", "session_id": "s1",
                "totals": {"tool_calls": 1}},
        "events": [
            {"seq": 0, "kind": "tool_call", "sidechain": False,
             "call_id": "c1", "tool": "Read",
             "timestamp": "2026-01-01T09:00:05.000Z"},
            {"seq": 1, "kind": "tool_result", "sidechain": False,
             "call_id": "c1", "timestamp": "2026-01-01T09:00:06.000Z"},
        ],
        "outcome": {"status": "completed"},
    }


def test_good_tape_is_valid():
    assert validate_tape(good_tape()) == []


def test_wrong_version_is_rejected():
    tape = good_tape()
    tape["tape_version"] = "9.9"
    assert any("tape_version" in d for d in validate_tape(tape))


def test_seq_gap_is_rejected():
    tape = good_tape()
    tape["events"][1]["seq"] = 7
    assert any("seq" in d for d in validate_tape(tape))


def test_orphan_tool_result_is_rejected():
    tape = good_tape()
    tape["events"][1]["call_id"] = "nope"
    assert any("answers no earlier" in d for d in validate_tape(tape))


def test_backward_timestamp_is_rejected():
    tape = good_tape()
    tape["events"][1]["timestamp"] = "2026-01-01T08:00:00.000Z"
    assert any("backward" in d for d in validate_tape(tape))


def test_duplicate_call_id_is_rejected():
    tape = good_tape()
    tape["events"].append({"seq": 2, "kind": "tool_call",
                           "sidechain": False, "call_id": "c1",
                           "tool": "Read"})
    assert any("used twice" in d for d in validate_tape(tape))


def test_bad_outcome_is_rejected():
    tape = good_tape()
    tape["outcome"]["status"] = "shrug"
    assert any("outcome.status" in d for d in validate_tape(tape))


def test_negative_total_is_rejected():
    tape = good_tape()
    tape["run"]["totals"]["tool_calls"] = -1
    assert any("totals" in d for d in validate_tape(tape))


def test_all_defects_are_named_in_one_pass():
    tape = good_tape()
    tape["tape_version"] = "9.9"
    tape["outcome"]["status"] = "shrug"
    tape["events"][1]["call_id"] = "nope"
    defects = validate_tape(tape)
    assert any("tape_version" in d for d in defects)
    assert any("outcome.status" in d for d in defects)
    assert any("answers no earlier" in d for d in defects)
    assert len(defects) >= 3


def test_skipped_lines_values_are_checked():
    tape = good_tape()
    tape["run"]["skipped_lines"] = {"system": -3}
    assert any("skipped_lines" in d for d in validate_tape(tape))
    tape["run"]["skipped_lines"] = {"system": "many"}
    assert any("skipped_lines" in d for d in validate_tape(tape))


def test_started_at_must_be_a_string():
    tape = good_tape()
    tape["run"]["started_at"] = 12345
    assert any("started_at" in d for d in validate_tape(tape))


def test_boolean_seq_is_rejected():
    tape = good_tape()
    tape["events"][1]["seq"] = True
    assert any("seq" in d for d in validate_tape(tape))


def test_integer_call_id_on_a_note_is_rejected():
    tape = good_tape()
    tape["events"].append({"seq": 2, "kind": "note", "sidechain": False,
                           "call_id": 7})
    assert any("call_id must be a string" in d for d in validate_tape(tape))


def test_boolean_total_is_rejected():
    tape = good_tape()
    tape["run"]["totals"]["tool_calls"] = True
    assert any("totals" in d for d in validate_tape(tape))


def test_non_boolean_sidechain_is_rejected():
    tape = good_tape()
    tape["events"][0]["sidechain"] = "yes"
    assert any("sidechain" in d for d in validate_tape(tape))


def test_offset_timestamps_compare_chronologically():
    tape = good_tape()
    tape["events"][0]["timestamp"] = "2026-01-01T10:00:00+02:00"
    tape["events"][1]["timestamp"] = "2026-01-01T09:00:00.000Z"
    assert validate_tape(tape) == []
    tape["events"][1]["timestamp"] = "2026-01-01T07:00:00.000Z"
    assert any("backward" in d for d in validate_tape(tape))


def test_unparseable_timestamp_is_rejected():
    tape = good_tape()
    tape["events"][0]["timestamp"] = "next tuesday"
    assert any("ISO 8601" in d for d in validate_tape(tape))


def test_basis_must_be_a_named_rule_that_pairs_with_the_status():
    tape = good_tape()
    tape["outcome"]["basis"] = "final-assistant-message"
    assert validate_tape(tape) == []
    for bad in ("", 7, None, ["x"], "banana", " "):
        tape = good_tape()
        tape["outcome"]["basis"] = bad
        assert any("outcome.basis" in d for d in validate_tape(tape)), bad
    tape = good_tape()
    tape["outcome"]["basis"] = "final-error-result"
    assert any("does not go with" in d for d in validate_tape(tape))
    tape = good_tape()
    tape["outcome"] = {"status": "unknown", "basis": "no-events"}
    assert validate_tape(tape) == []


def test_is_error_must_be_a_boolean_when_present():
    tape = good_tape()
    tape["events"][1]["is_error"] = True
    assert validate_tape(tape) == []
    tape = good_tape()
    tape["events"][1]["is_error"] = "false"
    assert any("is_error" in d for d in validate_tape(tape))


def test_a_non_finite_number_is_a_located_defect():
    tape = good_tape()
    tape["events"][0]["score"] = float("nan")
    defects = validate_tape(tape)
    assert any("non-finite" in d and "events[0]" in d for d in defects)


def test_a_non_finite_number_is_named_on_an_early_return_path():
    tape = good_tape()
    tape["events"] = "not a list"
    tape["run"]["cost"] = float("inf")
    defects = validate_tape(tape)
    assert any("non-finite" in d for d in defects)


def test_a_finite_tape_earns_no_non_finite_defect():
    assert not any("non-finite" in d for d in validate_tape(good_tape()))
