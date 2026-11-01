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
    tape = good_tape(); tape["tape_version"] = "9.9"
    assert any("tape_version" in d for d in validate_tape(tape))


def test_seq_gap_is_rejected():
    tape = good_tape(); tape["events"][1]["seq"] = 7
    assert any("seq" in d for d in validate_tape(tape))


def test_orphan_tool_result_is_rejected():
    tape = good_tape(); tape["events"][1]["call_id"] = "nope"
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
    tape = good_tape(); tape["outcome"]["status"] = "shrug"
    assert any("outcome.status" in d for d in validate_tape(tape))


def test_negative_total_is_rejected():
    tape = good_tape(); tape["run"]["totals"]["tool_calls"] = -1
    assert any("totals" in d for d in validate_tape(tape))


def test_all_defects_are_named_in_one_pass():
    tape = good_tape()
    tape["tape_version"] = "9.9"
    tape["outcome"]["status"] = "shrug"
    tape["events"][1]["call_id"] = "nope"
    defects = validate_tape(tape)
    assert len(defects) >= 3
