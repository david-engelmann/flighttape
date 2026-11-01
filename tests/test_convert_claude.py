import json, pathlib

from flighttape.convert_claude import convert_lines, convert_file
from flighttape.validate import validate_tape

FIXTURE = (pathlib.Path(__file__).parent / "fixtures"
           / "claude_session_synthetic.jsonl")


def fixture_tape():
    return convert_file(FIXTURE)


def test_converted_tape_validates():
    assert validate_tape(fixture_tape()) == []


def test_tool_call_keeps_source_id_and_pairs_with_result():
    tape = fixture_tape()
    calls = [e for e in tape["events"] if e["kind"] == "tool_call"]
    results = [e for e in tape["events"] if e["kind"] == "tool_result"]
    assert [c["call_id"] for c in calls] == ["toolu_synth_1"]
    assert [r["call_id"] for r in results] == ["toolu_synth_1"]
    assert calls[0]["tool"] == "Read"


def test_sidechain_event_is_preserved_and_flagged():
    tape = fixture_tape()
    side = [e for e in tape["events"] if e["sidechain"]]
    assert len(side) == 1
    assert "sidechain note" in side[0]["text"]


def test_usage_is_summed_into_run_totals():
    tape = fixture_tape()
    assert tape["run"]["totals"]["input_tokens"] == 150
    assert tape["run"]["totals"]["output_tokens"] == 50
    assert tape["run"]["totals"]["tool_calls"] == 1


def test_skipped_lines_are_counted_by_type():
    tape = fixture_tape()
    assert tape["run"]["skipped_lines"] == {"system": 1}


def test_unparseable_line_is_counted_not_fatal():
    tape = convert_lines(['{"type": "user", broken', json.dumps({
        "type": "user", "sessionId": "s9",
        "message": {"role": "user", "content": "hello"}})])
    assert tape["run"]["skipped_lines"]["unparseable"] == 1
    assert validate_tape(tape) == []


def test_tape_carries_no_source_paths():
    tape = fixture_tape()
    blob = json.dumps(tape)
    assert "cwd" not in blob and "gitBranch" not in blob
    assert tape["run"]["source_format"] == "claude-code-jsonl"
    assert tape["run"]["session_id"] == "synthetic-session-001"


def test_string_content_becomes_a_message_event():
    tape = convert_lines([json.dumps({
        "type": "user", "sessionId": "s2",
        "message": {"role": "user", "content": "plain string prompt"}})])
    kinds = [e["kind"] for e in tape["events"]]
    assert kinds == ["message"]
