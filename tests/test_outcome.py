"""Outcome determination tests (FT-14 to FT-16).

Each rule fires on a minimal event list built here, the sidechain
and session_end handling is probed directly, and the purity of the
function is checked by deep comparing the input before and after.
Converter level cases run small synthetic line sets through
convert_lines so the wiring is held as well as the function.
"""
import copy, json

import pytest

from flighttape.convert_claude import convert_lines
from flighttape.outcome import determine_outcome


def message(role, text="text", sidechain=False):
    return {"kind": "message", "role": role, "text": text,
            "sidechain": sidechain}


def tool_result(is_error=False, sidechain=False):
    return {"kind": "tool_result", "is_error": is_error,
            "sidechain": sidechain}


def tool_call(sidechain=False):
    return {"kind": "tool_call", "sidechain": sidechain}


def test_completed_on_final_assistant_message():
    events = [message("user"), message("assistant")]
    assert determine_outcome(events) == {
        "status": "completed", "basis": "final-assistant-message"}


def test_failed_on_final_error_result():
    events = [message("user"), tool_call(), tool_result(is_error=True)]
    assert determine_outcome(events) == {
        "status": "failed", "basis": "final-error-result"}


def test_recovered_error_is_not_failed():
    events = [tool_result(is_error=True), message("assistant")]
    assert determine_outcome(events)["status"] == "completed"


def test_aborted_on_the_interrupt_marker():
    events = [message("assistant"),
              message("user", "[Request interrupted by user]")]
    assert determine_outcome(events) == {
        "status": "aborted", "basis": "user-interrupt-marker"}
    events = [message("user", "[Request interrupted by user for tool use]")]
    assert determine_outcome(events)["status"] == "aborted"


def test_unknown_endings():
    assert determine_outcome([message("user")]) == {
        "status": "unknown", "basis": "unresolved-ending"}
    assert determine_outcome([tool_call()])["basis"] == "unresolved-ending"
    assert determine_outcome([tool_result()])["basis"] == "unresolved-ending"
    assert determine_outcome([{"kind": "note", "sidechain": False}])[
        "basis"] == "unresolved-ending"


def test_no_main_chain_events_is_unknown_no_events():
    assert determine_outcome([]) == {"status": "unknown", "basis": "no-events"}
    assert determine_outcome([message("assistant", sidechain=True)]) == {
        "status": "unknown", "basis": "no-events"}


def test_sidechain_ending_never_decides():
    events = [message("assistant"), tool_result(is_error=True, sidechain=True)]
    assert determine_outcome(events)["status"] == "completed"
    events = [tool_result(is_error=True), message("assistant", sidechain=True)]
    assert determine_outcome(events)["status"] == "failed"


def test_trailing_session_end_is_set_aside():
    end = {"kind": "session_end", "sidechain": False}
    events = [tool_result(is_error=True), end]
    assert determine_outcome(events)["status"] == "failed"
    assert determine_outcome([end]) == {
        "status": "unknown", "basis": "no-events"}
    events = [message("assistant"), end, dict(end)]
    assert determine_outcome(events)["status"] == "completed"


def test_determination_is_pure():
    events = [message("user"), tool_result(is_error=True),
              {"kind": "session_end", "sidechain": False}]
    snapshot = copy.deepcopy(events)
    first = determine_outcome(events)
    assert events == snapshot
    assert determine_outcome(events) == first


def _line(kind, content, **extra):
    line = {"type": kind, "sessionId": "synthetic-outcome",
            "message": {"role": kind, "content": content}}
    line.update(extra)
    return json.dumps(line)


def test_converter_records_a_completed_outcome():
    lines = [_line("user", "Synthetic prompt."),
             _line("assistant", "Synthetic answer.")]
    tape = convert_lines(lines)
    assert tape["outcome"] == {
        "status": "completed", "basis": "final-assistant-message"}


def test_converter_records_an_aborted_outcome():
    lines = [_line("user", "Synthetic prompt."),
             _line("assistant", "Synthetic partial answer."),
             _line("user", "[Request interrupted by user]")]
    tape = convert_lines(lines)
    assert tape["outcome"] == {
        "status": "aborted", "basis": "user-interrupt-marker"}


def test_marker_must_begin_the_text():
    quoted = message("user", "Quoting a log line, "
                             "[Request interrupted by user] appears here")
    assert determine_outcome([quoted]) == {
        "status": "unknown", "basis": "unresolved-ending"}
    other = message("user", "[Request interrupted by the system]")
    assert determine_outcome([other])["status"] == "unknown"
    spaced = message("user", " [Request interrupted by user]")
    assert determine_outcome([spaced])["status"] == "unknown"


def test_flags_read_as_booleans_only():
    events = [tool_result(is_error="false")]
    assert determine_outcome(events)["basis"] == "unresolved-ending"
    events = [tool_result(is_error=1)]
    assert determine_outcome(events)["basis"] == "unresolved-ending"
    events = [message("assistant", sidechain=1)]
    assert determine_outcome(events)["status"] == "completed"
    events = [message("assistant", sidechain="yes")]
    assert determine_outcome(events)["status"] == "completed"


def test_non_dict_entries_are_ignored():
    assert determine_outcome([None, 42, "x"]) == {
        "status": "unknown", "basis": "no-events"}
    assert determine_outcome([42, message("assistant")])[
        "status"] == "completed"


def test_final_user_message_without_string_text_is_unknown():
    no_text = {"kind": "message", "role": "user", "sidechain": False}
    assert determine_outcome([no_text])["status"] == "unknown"
    none_text = message("user", None)
    assert determine_outcome([none_text])["status"] == "unknown"


def test_non_message_events_with_roles_do_not_decide():
    note = {"kind": "note", "role": "assistant", "sidechain": False}
    assert determine_outcome([note])["status"] == "unknown"
    diff = {"kind": "diff", "role": "user", "sidechain": False,
            "text": "[Request interrupted by user]"}
    assert determine_outcome([diff])["status"] == "unknown"


def test_events_must_be_a_list():
    for bad in (None, 42, "tape", {"events": []}):
        with pytest.raises(TypeError):
            determine_outcome(bad)


def test_converter_records_a_failed_outcome():
    lines = [
        _line("assistant", [{"type": "tool_use", "id": "c1",
                             "name": "Read", "input": {}}]),
        _line("user", [{"type": "tool_result", "tool_use_id": "c1",
                        "is_error": True, "content": "Synthetic failure."}]),
    ]
    tape = convert_lines(lines)
    assert tape["outcome"] == {
        "status": "failed", "basis": "final-error-result"}
