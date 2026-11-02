"""Compare two tapes, the second against the first (FT-31 to FT-34).

The report is data for whoever runs before and after runs, a
person or another tool in the portfolio. Inputs must be valid
tapes; the command line validates before it compares, the
same refusal play makes.
"""

from .identity import canonical_bytes, tape_digest
from .validate import parse_timestamp


def _is_number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _pair(left, right):
    """One side by side entry, delta only when both sides are numbers."""
    entry = {}
    if _is_number(left):
        entry["a"] = left
    if _is_number(right):
        entry["b"] = right
    if _is_number(left) and _is_number(right):
        entry["delta"] = right - left
    return entry


def _measures(first, second):
    """A side by side entry for every numeric key in either mapping."""
    measures = {}
    for key in sorted(set(first) | set(second)):
        entry = _pair(first.get(key), second.get(key))
        if entry:
            measures[key] = entry
    return measures


def _duration(tape):
    """Seconds between the first and last parseable event timestamps."""
    stamps = []
    for event in tape["events"]:
        stamp = event.get("timestamp")
        if isinstance(stamp, str):
            parsed = parse_timestamp(stamp)
            if parsed is not None:
                stamps.append(parsed)
    if len(stamps) < 2:
        return None
    return (stamps[-1] - stamps[0]).total_seconds()


def _events(first, second):
    def by_seq(tape):
        return {event["seq"]: event for event in tape["events"]}

    def by_kind(tape):
        counts = {}
        for event in tape["events"]:
            counts[event["kind"]] = counts.get(event["kind"], 0) + 1
        return counts

    first_map, second_map = by_seq(first), by_seq(second)
    first_kinds, second_kinds = by_kind(first), by_kind(second)
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
    first_duration = _duration(first)
    second_duration = _duration(second)
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
        "measures": _measures(first["run"].get("totals", {}),
                              second["run"].get("totals", {})),
        "redacted_values": _pair(first["run"].get("redacted_values"),
                                 second["run"].get("redacted_values")),
        "skipped_lines": _measures(first["run"].get("skipped_lines", {}),
                                   second["run"].get("skipped_lines", {})),
        "events": _events(first, second),
    }
