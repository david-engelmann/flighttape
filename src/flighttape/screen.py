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

from .convert_claude import paths_in
from .identity import tape_digest

_TEXT_FIELDS = (("message", "text"), ("tool_result", "content"))


def _screen_string(text, seq, field, where, hits, totals):
    totals["strings"] += 1
    found = paths_in(text)
    if found:
        totals["paths"] += found
        hits.append({"seq": seq, "field": field, "in": where,
                     "paths": found})


def _children(node, seq, field, hits, totals):
    if isinstance(node, dict):
        for key, item in node.items():
            if isinstance(key, str):
                _screen_string(key, seq, field, "key",
                               hits, totals)
            yield item
    else:
        yield from node


def _screen_value(value, seq, field, hits, totals):
    """Screen every string in a carrier value (FT-45, FT-45a).

    The walk is iterative (FT-45b): an explicit stack of
    iterators, so no nesting depth a valid tape can hold
    exhausts the interpreter's recursion limit. A carrier
    holding a container is walked like an input, the way
    the converter's routing redacts it; the strings inside
    it are screened strings of that carrier's field.
    """
    if isinstance(value, str):
        _screen_string(value, seq, field, "value", hits, totals)
        return
    if not isinstance(value, (dict, list)):
        return
    stack = [iter(_children(value, seq, field, hits, totals))]
    while stack:
        try:
            child = next(stack[-1])
        except StopIteration:
            stack.pop()
            continue
        if isinstance(child, str):
            _screen_string(child, seq, field, "value",
                           hits, totals)
        elif isinstance(child, (dict, list)):
            stack.append(iter(_children(child, seq, field,
                                        hits, totals)))


def screen_tape(tape):
    """The screening report for one valid tape (FT-46)."""
    hits = []
    totals = {"strings": 0, "paths": 0}
    fields = dict(_TEXT_FIELDS)
    for event in tape["events"]:
        seq = event["seq"]
        field = fields.get(event.get("kind"))
        if field is not None:
            _screen_value(event.get(field), seq, field,
                          hits, totals)
        if event.get("kind") == "tool_call":
            _screen_value(event.get("input"), seq, "input",
                          hits, totals)
    return {"tape": tape_digest(tape),
            "strings_screened": totals["strings"],
            "paths_found": totals["paths"],
            "hits": hits,
            "clean": totals["paths"] == 0}
