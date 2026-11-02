"""Tape screening (FT-44 to FT-48).

Conversion redacts every absolute path out of the text
it emits (FT-22, FT-23); screening is the same rule read
in the other direction. A tape from any source is walked
over exactly the strings FT-22 names as text, each one
run through the converter's own redactor, and every
string the redactor would change is reported as a hit:
the event's seq, the field, whether the paths sat in a
key or in a value, and how many paths the string holds.
The report never carries a screened string or any part
of a path it found, so the report itself is safe to
share, and a clean screen is the check a tape passes
before it enters a shared set.
"""
from __future__ import annotations

from .convert_claude import _redact_text
from .identity import tape_digest

_TEXT_FIELDS = (("message", "text"), ("tool_result", "content"))


def _paths_in(text):
    counter = [0]
    _redact_text(text, counter)
    return counter[0]


def _screen_string(text, seq, field, where, hits, totals):
    totals["strings"] += 1
    found = _paths_in(text)
    if found:
        totals["paths"] += found
        hits.append({"seq": seq, "field": field, "in": where,
                     "paths": found})


def _screen_value(value, seq, field, hits, totals):
    if isinstance(value, str):
        _screen_string(value, seq, field, "value", hits, totals)
    elif isinstance(value, dict):
        for key, item in value.items():
            if isinstance(key, str):
                _screen_string(key, seq, field, "key",
                               hits, totals)
            _screen_value(item, seq, field, hits, totals)
    elif isinstance(value, list):
        for item in value:
            _screen_value(item, seq, field, hits, totals)


def screen_tape(tape):
    """The screening report for one valid tape (FT-46)."""
    hits: list = []
    totals = {"strings": 0, "paths": 0}
    fields = dict(_TEXT_FIELDS)
    for event in tape["events"]:
        seq = event["seq"]
        field = fields.get(event.get("kind"))
        if field is not None:
            value = event.get(field)
            if isinstance(value, str):
                _screen_string(value, seq, field, "value",
                               hits, totals)
        if event.get("kind") == "tool_call":
            _screen_value(event.get("input"), seq, "input",
                          hits, totals)
    return {"tape": tape_digest(tape),
            "strings_screened": totals["strings"],
            "paths_found": totals["paths"],
            "hits": hits,
            "clean": totals["paths"] == 0}
