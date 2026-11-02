"""Compare two tapes, the second against the first (FT-31 to FT-34).

The report is data for the people and tools who compare runs,
before against after. Inputs must be valid tapes; the command
line validates before it compares, and refuses a side it
cannot digest, since the report carries digests.
"""

import math

from .identity import canonical_bytes, tape_digest
from .validate import is_number, tape_duration


def _delta(left, right):
    """The difference, or None when it is not a finite number.

    A huge integer against a float overflows the subtraction,
    and distant floats can differ by more than a double holds;
    the report is JSON, so an unrepresentable delta is omitted
    rather than emitted as a non-standard value.
    """
    try:
        delta = right - left
    except OverflowError:
        return None
    if isinstance(delta, float) and not math.isfinite(delta):
        return None
    return delta


def _entry(left, right):
    """One side by side entry, delta only when both sides are numbers."""
    entry = {}
    if is_number(left):
        entry["a"] = left
    if is_number(right):
        entry["b"] = right
    if is_number(left) and is_number(right):
        delta = _delta(left, right)
        if delta is not None:
            entry["delta"] = delta
    return entry


def _measures(first, second):
    """A side by side entry for every numeric key in either mapping."""
    measures = {}
    for key in sorted(set(first) | set(second)):
        entry = _entry(first.get(key), second.get(key))
        if entry:
            measures[key] = entry
    return measures


def _carried(first_run, second_run):
    """The redacted_values entry, each side's value as it stands (FT-32).

    The validator does not constrain this field's type, so the
    report carries the value itself, with a delta only when
    both sides are numbers.
    """
    entry = {}
    if "redacted_values" in first_run:
        entry["a"] = first_run["redacted_values"]
    if "redacted_values" in second_run:
        entry["b"] = second_run["redacted_values"]
    if is_number(entry.get("a")) and is_number(entry.get("b")):
        delta = _delta(entry["a"], entry["b"])
        if delta is not None:
            entry["delta"] = delta
    return entry


def _by_seq(tape):
    return {event["seq"]: event for event in tape["events"]}


def _by_kind(tape):
    counts = {}
    for event in tape["events"]:
        counts[event["kind"]] = counts.get(event["kind"], 0) + 1
    return counts


def _events(first, second):
    first_map, second_map = _by_seq(first), _by_seq(second)
    first_kinds, second_kinds = _by_kind(first), _by_kind(second)
    return {
        "count": {"a": len(first["events"]), "b": len(second["events"])},
        "by_kind": {
            kind: {"a": first_kinds.get(kind, 0),
                   "b": second_kinds.get(kind, 0)}
            for kind in sorted(set(first_kinds) | set(second_kinds))
        },
        "changed": sorted(
            seq for seq in first_map.keys() & second_map.keys()
            if canonical_bytes(first_map[seq])
            != canonical_bytes(second_map[seq])),
        "added": sorted(second_map.keys() - first_map.keys()),
        "removed": sorted(first_map.keys() - second_map.keys()),
    }


def compare_tapes(first, second):
    """Return the comparison report for two valid tapes."""
    first_outcome, second_outcome = first["outcome"], second["outcome"]
    first_duration = tape_duration(first)
    second_duration = tape_duration(second)
    duration = {"a": first_duration, "b": second_duration}
    if first_duration is not None and second_duration is not None:
        duration["delta"] = second_duration - first_duration
    return {
        "digests": {"a": tape_digest(first), "b": tape_digest(second)},
        "outcome": {
            "a": {"status": first_outcome["status"],
                  "basis": first_outcome.get("basis")},
            "b": {"status": second_outcome["status"],
                  "basis": second_outcome.get("basis")},
            "changed": (first_outcome["status"],
                        first_outcome.get("basis"))
                       != (second_outcome["status"],
                           second_outcome.get("basis")),
        },
        "duration_seconds": duration,
        "measures": _measures(first["run"].get("totals") or {},
                              second["run"].get("totals") or {}),
        "redacted_values": _carried(first["run"], second["run"]),
        "skipped_lines": _measures(
            first["run"].get("skipped_lines") or {},
            second["run"].get("skipped_lines") or {}),
        "events": _events(first, second),
    }
