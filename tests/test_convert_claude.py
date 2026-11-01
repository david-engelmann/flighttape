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
    assert tape["run"]["totals"]["messages"] == 3


def test_cache_tokens_are_summed_under_their_own_keys():
    tape = convert_lines([json.dumps({
        "type": "assistant", "sessionId": "s3",
        "message": {"role": "assistant",
                    "usage": {"input_tokens": 100, "output_tokens": 10,
                              "cache_creation_input_tokens": 500,
                              "cache_read_input_tokens": 900},
                    "content": [{"type": "text", "text": "hi"}]}})])
    assert tape["run"]["totals"]["cache_creation_input_tokens"] == 500
    assert tape["run"]["totals"]["cache_read_input_tokens"] == 900


def test_tool_result_content_is_carried_into_the_tape():
    tape = fixture_tape()
    results = [e for e in tape["events"] if e["kind"] == "tool_result"]
    assert results[0]["content"] == "counter = 0"


def test_tool_result_error_flag_is_preserved():
    tape = convert_lines([
        json.dumps({"type": "assistant", "sessionId": "s4",
                    "message": {"role": "assistant", "content": [
                        {"type": "tool_use", "id": "t1", "name": "Bash",
                         "input": {}}]}}),
        json.dumps({"type": "user", "sessionId": "s4",
                    "message": {"role": "user", "content": [
                        {"type": "tool_result", "tool_use_id": "t1",
                         "is_error": True, "content": "boom"}]}})])
    results = [e for e in tape["events"] if e["kind"] == "tool_result"]
    assert results[0]["is_error"] is True
    assert results[0]["content"] == "boom"


def test_events_keep_timestamps_and_run_keeps_start():
    tape = fixture_tape()
    assert tape["run"]["started_at"] == "2026-01-01T09:00:00.000Z"
    assert tape["events"][0]["timestamp"] == "2026-01-01T09:00:00.000Z"


def test_non_object_and_malformed_lines_are_counted_not_fatal():
    tape = convert_lines(["42", "[1,2,3]", "null", json.dumps({
        "type": "user", "sessionId": "s5", "message": "oops"})])
    assert tape["run"]["skipped_lines"]["non-object"] == 3
    assert tape["run"]["skipped_lines"]["malformed:message"] == 1
    assert tape["events"] == []


def test_second_session_lines_are_counted_and_skipped():
    tape = convert_lines([
        json.dumps({"type": "user", "sessionId": "first",
                    "message": {"role": "user", "content": "one"}}),
        json.dumps({"type": "user", "sessionId": "second",
                    "message": {"role": "user", "content": "two"}})])
    assert tape["run"]["session_id"] == "first"
    assert tape["run"]["skipped_lines"]["foreign-session"] == 1
    assert len(tape["events"]) == 1


def test_contentless_lines_are_counted():
    tape = convert_lines([json.dumps({
        "type": "user", "sessionId": "s6",
        "message": {"role": "user", "content": []}})])
    assert tape["run"]["skipped_lines"]["contentless:user"] == 1
    assert tape["events"] == []


def test_empty_conversion_fabricates_no_event():
    tape = convert_lines([])
    assert tape["events"] == []
    assert validate_tape(tape) != []


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
    assert "/home/synthetic" not in blob
    assert "synthetic-branch" not in blob
    assert "cwd" not in blob and "gitBranch" not in blob
    assert tape["run"]["source_format"] == "claude-code-jsonl"
    assert tape["run"]["session_id"] == "synthetic-session-001"


def test_absolute_paths_in_tool_inputs_are_reduced_and_counted():
    tape = convert_lines([json.dumps({
        "type": "assistant", "sessionId": "s7",
        "message": {"role": "assistant", "content": [
            {"type": "tool_use", "id": "t9", "name": "Read",
             "input": {"file_path": "/home/synthetic/private/notes.txt"}}]}})])
    blob = json.dumps(tape)
    assert "/home/synthetic" not in blob
    assert "notes.txt" in blob
    assert tape["run"]["redacted_values"] == 1


def test_string_content_becomes_a_message_event():
    tape = convert_lines([json.dumps({
        "type": "user", "sessionId": "s2",
        "message": {"role": "user", "content": "plain string prompt"}})])
    kinds = [e["kind"] for e in tape["events"]]
    assert kinds == ["message"]
