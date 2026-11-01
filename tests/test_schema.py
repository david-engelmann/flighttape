import json, pathlib

from flighttape import TAPE_VERSION
from flighttape.validate import EVENT_KINDS, OUTCOMES, validate_tape

SCHEMA = (pathlib.Path(__file__).resolve().parent.parent
          / "schema" / "tape.schema.json")


def load_schema():
    return json.loads(SCHEMA.read_text(encoding="utf-8"))


def test_schema_and_validator_share_the_vocabulary():
    schema = load_schema()
    assert schema["properties"]["tape_version"]["const"] == TAPE_VERSION
    kinds = schema["properties"]["events"]["items"]["properties"]["kind"]["enum"]
    assert set(kinds) == EVENT_KINDS
    outcomes = schema["properties"]["outcome"]["properties"]["status"]["enum"]
    assert set(outcomes) == OUTCOMES


def test_schema_requires_non_empty_call_and_tool():
    schema = load_schema()
    props = schema["properties"]["events"]["items"]["properties"]
    assert props["call_id"]["minLength"] == 1
    assert props["tool"]["minLength"] == 1


def test_validator_rejects_the_schema_probes():
    base = {
        "tape_version": "1.0",
        "run": {"harness": "claude-code", "session_id": "s1"},
        "events": [{"seq": 0, "kind": "note", "sidechain": False}],
        "outcome": {"status": "completed"},
    }
    assert validate_tape(base) == []
    import copy
    probe = copy.deepcopy(base)
    probe["run"]["skipped_lines"] = {"system": -3}
    assert validate_tape(probe) != []
    probe = copy.deepcopy(base)
    probe["run"]["started_at"] = 12345
    assert validate_tape(probe) != []
    probe = copy.deepcopy(base)
    probe["events"][0]["call_id"] = 7
    assert validate_tape(probe) != []
