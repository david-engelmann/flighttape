"""Outcome determination tests (FT-14 to FT-16).

Each rule fires on a minimal event list built here, the sidechain
and session_end handling is probed directly, and the purity of the
function is checked by deep comparing the input before and after.
Converter level cases run small synthetic line sets through
convert_lines so the wiring is held as well as the function.
"""
import copy, json

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
