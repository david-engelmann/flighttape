"""Roll a directory of tapes up into one report (FT-35 to FT-38).

The report is the set level record the retrospective tools
consume: which tapes are in the set, what each one holds,
and the sums across them. Only valid, digestible tapes are
rolled up; every other file is recorded as an exclusion with
its reason, and aggregates cover the rolled tapes only.
"""

import math

from .validate import parse_timestamp


def _is_number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _duration(tape):
    """Seconds between the first and last parseable timestamps.

    The same duration rule compare applies to one tape
    (FT-32), null below two parseable timestamps.
    """
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


def _redacted_total(values):
    """The sum of the numeric redacted_values, or None.

    The field is unconstrained, so the sum can overflow a
    double (a huge integer among floats) or run past finite;
    the report is JSON, so an unrepresentable total is null
    rather than a non-standard value.
    """
    total = 0
    for value in values:
        try:
            total += value
        except OverflowError:
            return None
        if isinstance(total, float) and not math.isfinite(total):
            return None
    return total


def rollup_report(records, directory):
    """Build the rollup report from classified records.

    A record is either a rolled tape, {"file", "tape",
    "digest"}, or an exclusion, {"file", "reason", plus
    "defects" or "error" where the reason carries them}.
    Records may arrive in any order; the report lists tapes
    and exclusions in filename order.
    """
    rolled = sorted((record for record in records if "tape" in record),
                    key=lambda record: record["file"])
    excluded = sorted((record for record in records
                       if "tape" not in record),
                      key=lambda record: record["file"])
    entries = []
    outcomes = {}
    totals = {}
    skipped = {}
    redacted_values = []
    redacted_non_numeric = 0
    duration_total = 0.0
    duration_known = 0
    event_count = 0
    for record in rolled:
        tape = record["tape"]
        run = tape["run"]
        duration = _duration(tape)
        tape_totals = run.get("totals") or {}
        entries.append({
            "file": record["file"],
            "digest": record["digest"],
            "outcome": {"status": tape["outcome"]["status"],
                        "basis": tape["outcome"].get("basis")},
            "duration_seconds": duration,
            "events": len(tape["events"]),
            "totals": dict(tape_totals),
        })
        event_count += len(tape["events"])
        status = tape["outcome"]["status"]
        outcomes[status] = outcomes.get(status, 0) + 1
        for key, value in tape_totals.items():
            if _is_number(value):
                totals[key] = totals.get(key, 0) + value
        for key, value in (run.get("skipped_lines") or {}).items():
            if _is_number(value):
                skipped[key] = skipped.get(key, 0) + value
        if "redacted_values" in run:
            if _is_number(run["redacted_values"]):
                redacted_values.append(run["redacted_values"])
            else:
                redacted_non_numeric += 1
        if duration is not None:
            duration_total += duration
            duration_known += 1
    return {
        "directory": directory,
        "tapes": entries,
        "excluded": [dict(record) for record in excluded],
        "counts": {"tapes": len(entries), "excluded": len(excluded),
                   "events": event_count},
        "outcomes": {key: outcomes[key] for key in sorted(outcomes)},
        "totals": {key: totals[key] for key in sorted(totals)},
        "redacted_values": {
            "total": _redacted_total(redacted_values),
            "non_numeric": redacted_non_numeric,
        },
        "skipped_lines": {key: skipped[key] for key in sorted(skipped)},
        "duration_seconds": {"total": duration_total,
                             "known": duration_known,
                             "unknown": len(entries) - duration_known},
    }
