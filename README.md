# flighttape

Every agent run, from every harness, lands as one flight tape: events, tool calls, diffs, spend, timings, and the outcome, in a bundle any tool can read back.

Flighttape defines one bundle format for a finished agent run, and the tools that work over it. The format is specified in CONTRACT.md and in the schema at schema/tape.schema.json. A harness session file becomes a tape once, and every reader after that works on the tape instead of reverse engineering each harness on its own. The converter reads Claude Code session files. Flighttape records the run, onrecord remembers across runs.

![ci](https://github.com/david-engelmann/flighttape/actions/workflows/ci.yml/badge.svg)

```text
tape readme-example-001 from claude-code
outcome completed (final-assistant-message), events 5, tool calls 1, messages 3, input tokens 135, output tokens 40, cache creation input tokens 0, cache read input tokens 0
started 2026-02-02T10:00:00.000Z, source claude-code-jsonl

[+0.00s] #0 message user
Synthetic prompt, sort the example records by name.
[+4.00s] #1 message assistant
Synthetic reply, reading the records first.
[+4.00s] #2 tool_call Read call toolu_readme_1
{"path": "records.txt"}
[+5.00s] #3 tool_result call toolu_readme_1
beta
alpha

[+20.0s] #4 message assistant
Synthetic reply, the sorted order is alpha then beta.
```

## Install

```sh
git clone https://github.com/david-engelmann/flighttape.git
cd flighttape
cargo install --path .
```

The install needs Git and a Rust toolchain.

## Quickstart

The repository ships a small synthetic Claude Code session for trying the tool. Convert it to a tape, then check the tape and print its digest.

```sh
flighttape convert tests/fixtures/claude_session_synthetic.jsonl -o example.tape.json
flighttape validate example.tape.json
valid tape, 5 events
flighttape digest example.tape.json
cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3
```

## Usage

The examples below run in a working directory staged from the two small synthetic session files shipped with this repository. Copy examples/session-a.jsonl and examples/session-b.jsonl into a new directory, and the same two files into a subdirectory of it named sessions, and run the commands there. Every output is captured from the tool itself.

Convert the two session files into a directory named tapes.

```sh
mkdir tapes
flighttape convert session-a.jsonl -o tapes/run-a.tape.json
flighttape convert session-b.jsonl -o tapes/run-b.tape.json
```

A whole directory of sessions converts in one run with convert-dir. It names each tape after the session it came from and writes a summary file named summary.json beside the tapes. A rollup over that directory counts the summary as excluded and exits 1, because the summary is not a tape.

```sh
flighttape convert-dir sessions -o converted
converted 2, failed 0, events 10
```

Validate a tape, print its digest, and check a tape against a digest someone cites.

```sh
flighttape validate tapes/run-a.tape.json
valid tape, 5 events
flighttape digest tapes/run-a.tape.json
a88422b41393c0f6be88b624ce7595c1615af3c8eb0af3ae3d15dda622080e79
flighttape verify tapes/run-a.tape.json a88422b41393c0f6be88b624ce7595c1615af3c8eb0af3ae3d15dda622080e79
digest matches, a88422b41393c0f6be88b624ce7595c1615af3c8eb0af3ae3d15dda622080e79
```

Play a tape back as a readable timeline.

```sh
flighttape play tapes/run-a.tape.json
tape readme-example-001 from claude-code
outcome completed (final-assistant-message), events 5, tool calls 1, messages 3, input tokens 135, output tokens 40, cache creation input tokens 0, cache read input tokens 0
started 2026-02-02T10:00:00.000Z, source claude-code-jsonl

[+0.00s] #0 message user
Synthetic prompt, sort the example records by name.
[+4.00s] #1 message assistant
Synthetic reply, reading the records first.
[+4.00s] #2 tool_call Read call toolu_readme_1
{"path": "records.txt"}
[+5.00s] #3 tool_result call toolu_readme_1
beta
alpha

[+20.0s] #4 message assistant
Synthetic reply, the sorted order is alpha then beta.
```

Compare two tapes, roll up every tape in a directory, cut a slice out of a tape, and screen a tape for absolute paths before sharing it.

```sh
flighttape compare tapes/run-a.tape.json tapes/run-b.tape.json
{
  "digests": {
    "a": "a88422b41393c0f6be88b624ce7595c1615af3c8eb0af3ae3d15dda622080e79",
    "b": "90dbaa3f361d39e19bce0c5b08781b17bfa3673692704d688f381c1cd2a94d61"
  },
  "outcome": {
    "a": {
      "status": "completed",
      "basis": "final-assistant-message"
    },
    "b": {
      "status": "completed",
      "basis": "final-assistant-message"
    },
    "changed": false
  },
  "duration_seconds": {
    "a": 20.0,
    "b": 20.0,
    "delta": 0.0
  },
  "measures": {
    "cache_creation_input_tokens": {
      "a": 0,
      "b": 0,
      "delta": 0
    },
    "cache_read_input_tokens": {
      "a": 0,
      "b": 0,
      "delta": 0
    },
    "input_tokens": {
      "a": 135,
      "b": 135,
      "delta": 0
    },
    "messages": {
      "a": 3,
      "b": 3,
      "delta": 0
    },
    "output_tokens": {
      "a": 40,
      "b": 40,
      "delta": 0
    },
    "tool_calls": {
      "a": 1,
      "b": 1,
      "delta": 0
    }
  },
  "redacted_values": {
    "a": 0,
    "b": 0,
    "delta": 0
  },
  "skipped_lines": {},
  "events": {
    "count": {
      "a": 5,
      "b": 5
    },
    "by_kind": {
      "message": {
        "a": 3,
        "b": 3
      },
      "tool_call": {
        "a": 1,
        "b": 1
      },
      "tool_result": {
        "a": 1,
        "b": 1
      }
    },
    "changed": [
      0,
      1,
      2,
      3,
      4
    ],
    "added": [],
    "removed": []
  }
}
flighttape rollup tapes
{
  "directory": "tapes",
  "tapes": [
    {
      "file": "run-a.tape.json",
      "digest": "a88422b41393c0f6be88b624ce7595c1615af3c8eb0af3ae3d15dda622080e79",
      "outcome": {
        "status": "completed",
        "basis": "final-assistant-message"
      },
      "duration_seconds": 20.0,
      "events": 5,
      "totals": {
        "input_tokens": 135,
        "output_tokens": 40,
        "cache_creation_input_tokens": 0,
        "cache_read_input_tokens": 0,
        "tool_calls": 1,
        "messages": 3
      }
    },
    {
      "file": "run-b.tape.json",
      "digest": "90dbaa3f361d39e19bce0c5b08781b17bfa3673692704d688f381c1cd2a94d61",
      "outcome": {
        "status": "completed",
        "basis": "final-assistant-message"
      },
      "duration_seconds": 20.0,
      "events": 5,
      "totals": {
        "input_tokens": 135,
        "output_tokens": 40,
        "cache_creation_input_tokens": 0,
        "cache_read_input_tokens": 0,
        "tool_calls": 1,
        "messages": 3
      }
    }
  ],
  "excluded": [],
  "counts": {
    "tapes": 2,
    "excluded": 0,
    "events": 10
  },
  "outcomes": {
    "completed": 2
  },
  "totals": {
    "cache_creation_input_tokens": 0,
    "cache_read_input_tokens": 0,
    "input_tokens": 270,
    "messages": 6,
    "output_tokens": 80,
    "tool_calls": 2
  },
  "redacted_values": {
    "total": 0,
    "non_numeric": 0
  },
  "skipped_lines": {},
  "duration_seconds": {
    "total": 40.0,
    "known": 2,
    "unknown": 0
  }
}
flighttape slice tapes/run-a.tape.json --from 0 --to 1 -o opening.tape.json
flighttape screen tapes/run-a.tape.json
{
  "tape": "a88422b41393c0f6be88b624ce7595c1615af3c8eb0af3ae3d15dda622080e79",
  "strings_screened": 6,
  "paths_found": 0,
  "hits": [],
  "clean": true
}
```

## Status

The tape format and all ten commands are built and run as a Rust binary, with the Python of the first ten rounds standing as the behavior reference the port is pinned against. The converter reads Claude Code session files. Converters for other harnesses are not built yet, and the format is the part that is meant to outlast any one harness.

Closed perfection rounds, 26 of 100. A round counts only when a full cycle of work, audit, review, red team, and approval has closed.

## License

MIT. See LICENSE.
