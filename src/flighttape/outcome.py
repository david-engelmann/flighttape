"""Determine a tape's outcome from its events (FT-14 to FT-16).

The determination reads the main chain only, meaning events not
flagged sidechain, because a sidechain is a subagent's work and its
ending does not end the run. Trailing session_end events are set
aside first, because they mark the end of the recording rather
than the end of the run. The rules then read the final main chain
event, and the outcome records the basis, the name of the rule
that fired, so the determination can be audited without rerunning
it. The function is pure, the same events always give the same
outcome, with no clock, no file access, and no mutation of the
input. The input is the events list of a tape, anything else
raises TypeError, and non-object entries in the list are ignored.
The sidechain and is_error flags are booleans, and only the value
True reads as set.
"""
from __future__ import annotations

INTERRUPT_MARKER = "[Request interrupted by user"


def determine_outcome(events):
    """Return the outcome object for a tape's events list."""
    if not isinstance(events, list):
        raise TypeError("events must be a list")
    main = [event for event in events
            if isinstance(event, dict)
            and event.get("sidechain") is not True]
    while main and main[-1].get("kind") == "session_end":
        main.pop()
    if not main:
        return {"status": "unknown", "basis": "no-events"}
    final = main[-1]
    kind = final.get("kind")
    if kind == "message" and final.get("role") == "user":
        text = final.get("text")
        if isinstance(text, str) and text.startswith(INTERRUPT_MARKER):
            return {"status": "aborted", "basis": "user-interrupt-marker"}
    if kind == "tool_result" and final.get("is_error") is True:
        return {"status": "failed", "basis": "final-error-result"}
    if kind == "message" and final.get("role") == "assistant":
        return {"status": "completed", "basis": "final-assistant-message"}
    return {"status": "unknown", "basis": "unresolved-ending"}
