"""Roll a directory of tapes up into one report (FT-35 to FT-38).

The report is the set level record the retrospective tools
consume: which tapes are in the set, what each one holds,
and the sums across them. Only valid, digestible tapes are
rolled up; every other file is recorded as an exclusion with
its reason, and aggregates cover the rolled tapes only.
"""

import math
import sys

from .validate import is_number, tape_duration


def _renderable(total):
    """The sum, or None when JSON cannot render it.

    A float sum can run past finite, and an integer sum can
    pass the interpreter's digit limit for rendering numbers,
    the same limit the loader enforces per value, which sums
    across tapes can cross. The report is JSON, so an
    unrenderable sum is null rather than a crash at print
    time (FT-36 as amended).
    """
    if isinstance(total, float) and not math.isfinite(total):
        return None
    limit = sys.get_int_max_str_digits()
    if (limit and isinstance(total, int)
            and abs(total) >= 10 ** limit):
        return None
    return total


def _classify(record):
    """The kind of one record, "tape" or "exclusion".

    A record is a rolled tape or an exclusion, never both
    and never neither. Any other shape is a caller's
    programming error and raises ValueError naming the file,
    rather than silently miscounting the set or crashing on
    a missing key.
    """
    name = record.get("file", "<unnamed>")
    if "tape" in record and "reason" in record:
        raise ValueError(f"record for {name} is both a tape "
                         "and an exclusion")
    if "tape" in record:
        if not isinstance(record["tape"], dict):
            raise ValueError(f"rolled record for {name} holds "
                             "no tape")
        if "digest" not in record:
            raise ValueError(f"rolled record for {name} holds "
                             "no digest")
        return "tape"
    if "reason" in record:
        return "exclusion"
    raise ValueError(f"record for {name} is neither a tape "
                     "nor an exclusion")


def _tape_entry(record):
    tape = record["tape"]
    return {
        "file": record["file"],
        "digest": record["digest"],
        "outcome": {"status": tape["outcome"]["status"],
                    "basis": tape["outcome"].get("basis")},
        "duration_seconds": tape_duration(tape),
        "events": len(tape["events"]),
        "totals": dict(tape["run"].get("totals") or {}),
    }


def _exclusion_entry(record):
    entry = {"file": record["file"], "reason": record["reason"]}
    if "defects" in record:
        entry["defects"] = list(record["defects"])
    if "error" in record:
        entry["error"] = record["error"]
    return entry


def _sum_maps(maps):
    """Sum the numeric values per key across mappings."""
    sums = {}
    unrenderable = set()
    for mapping in maps:
        for key, value in mapping.items():
            if not is_number(value) or key in unrenderable:
                continue
            try:
                sums[key] = sums.get(key, 0) + value
            except OverflowError:
                unrenderable.add(key)
                sums[key] = None
    return {key: _renderable(sums[key]) for key in sorted(sums)}


def _redacted_summary(runs):
    """The redacted_values aggregate over the rolled runs."""
    total = 0
    overflowed = False
    non_numeric = 0
    for run in runs:
        if "redacted_values" not in run:
            continue
        value = run["redacted_values"]
        if not is_number(value):
            non_numeric += 1
        elif not overflowed:
            try:
                total += value
            except OverflowError:
                overflowed = True
    return {"total": None if overflowed else _renderable(total),
            "non_numeric": non_numeric}


def rollup_report(records, directory):
    """Build the rollup report from classified records.

    A record is either a rolled tape, {"file", "tape",
    "digest"}, or an exclusion, {"file", "reason", plus
    "defects" or "error" where the reason carries them}.
    Records may arrive in any order; filename order in the
    report is this function's guarantee, not the caller's.
    """
    rolled = []
    excluded = []
    for record in records:
        if _classify(record) == "tape":
            rolled.append(record)
        else:
            excluded.append(record)
    rolled.sort(key=lambda record: record["file"])
    excluded.sort(key=lambda record: record["file"])
    entries = [_tape_entry(record) for record in rolled]
    runs = [record["tape"]["run"] for record in rolled]
    outcomes = {}
    for entry in entries:
        status = entry["outcome"]["status"]
        outcomes[status] = outcomes.get(status, 0) + 1
    durations = [entry["duration_seconds"] for entry in entries]
    known = [value for value in durations if value is not None]
    return {
        "directory": directory,
        "tapes": entries,
        "excluded": [_exclusion_entry(record) for record in excluded],
        "counts": {"tapes": len(entries), "excluded": len(excluded),
                   "events": sum(entry["events"] for entry in entries)},
        "outcomes": {key: outcomes[key] for key in sorted(outcomes)},
        "totals": _sum_maps([entry["totals"] for entry in entries]),
        "redacted_values": _redacted_summary(runs),
        "skipped_lines": _sum_maps(
            [run.get("skipped_lines") or {} for run in runs]),
        "duration_seconds": {"total": sum(known, 0.0),
                             "known": len(known),
                             "unknown": len(durations) - len(known)},
    }
