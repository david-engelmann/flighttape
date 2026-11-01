"""Redaction completeness tests (FT-22 to FT-25).

Every session line in this file is synthetic, and the paths in
them are invented. The round's claim is that no absolute path in
any text the converter emits survives into the tape, that the
count in run.redacted_values is exactly the number of paths
replaced, and that text holding no absolute path is emitted
byte-identically.
"""
import json, pathlib

from flighttape.convert_claude import convert_file, convert_lines

FIXTURE = (pathlib.Path(__file__).resolve().parent / "fixtures"
           / "claude_session_synthetic.jsonl")
SECRET = "/home/synthetic-user/projects/secret-plan.md"


def line(kind, content, session="synthetic-redact"):
    return json.dumps({
        "type": kind, "sessionId": session,
        "timestamp": "2026-01-01T00:00:00Z",
        "message": {"role": kind, "content": content}})


def user_text(text):
    return line("user", text)


def assistant_text(text):
    return line("assistant", [{"type": "text", "text": text}])


def tool_use(block_input, name="Read"):
    return line("assistant", [{"type": "tool_use", "id": "c1",
                               "name": name, "input": block_input}])


def tool_result(content):
    return line("user", [{"type": "tool_result", "tool_use_id": "c1",
                          "content": content}])


def test_paths_in_every_text_field_are_redacted_and_counted():
    tape = convert_lines([
        user_text(f"please read {SECRET} now"),
        tool_use({"file_path": SECRET}),
        tool_result(f"contents of {SECRET} follow"),
        tool_use({"command": f"cat {SECRET} | wc -l"}, name="Bash"),
        assistant_text(f"I read {SECRET} and it says hello"),
    ])
    assert "/home/synthetic-user" not in json.dumps(tape)
    assert tape["run"]["redacted_values"] == 5
    texts = [event.get("text") or event.get("content") or ""
             for event in tape["events"]]
    assert "please read secret-plan.md now" in texts
    assert "contents of secret-plan.md follow" in texts
    assert "I read secret-plan.md and it says hello" in texts
    calls = [event for event in tape["events"]
             if event["kind"] == "tool_call"]
    assert calls[0]["input"] == {"file_path": "secret-plan.md"}
    assert calls[1]["input"] == {"command": "cat secret-plan.md | wc -l"}


def test_a_path_nested_in_a_tool_input_is_redacted():
    tape = convert_lines([tool_use(
        {"args": ["run", {"script": f"python {SECRET}"}]})])
    call = tape["events"][0]
    assert call["input"] == {"args": ["run", {"script": "python secret-plan.md"}]}
    assert tape["run"]["redacted_values"] == 1


def test_two_paths_in_one_text_are_each_counted():
    tape = convert_lines([user_text(
        "compare /home/synthetic-user/a/first.md with"
        " /home/synthetic-user/b/second.md today")])
    assert tape["events"][0]["text"] == "compare first.md with second.md today"
    assert tape["run"]["redacted_values"] == 2


def test_boundary_shapes_keep_their_punctuation():
    cases = [
        (f'open "{SECRET}" please', 'open "secret-plan.md" please'),
        (f"open '{SECRET}' please", "open 'secret-plan.md' please"),
        (f"run `{SECRET}` now", "run `secret-plan.md` now"),
        (f"(see {SECRET})", "(see secret-plan.md)"),
        (f"I read {SECRET}.", "I read secret-plan.md."),
        (f"I read {SECRET}, then left", "I read secret-plan.md, then left"),
        (f"file={SECRET}", "file=secret-plan.md"),
        (f"path:{SECRET}", "path:secret-plan.md"),
        (f"{SECRET}", "secret-plan.md"),
        ("/home/synthetic-user/projects/", "projects"),
    ]
    for given, expected in cases:
        tape = convert_lines([user_text(given)])
        assert tape["events"][0]["text"] == expected, given
        assert tape["run"]["redacted_values"] == 1, given


def test_text_without_absolute_paths_is_byte_identical():
    samples = [
        "the and/or choice stays, as does input/output",
        "on 2026/11/01 the ratio was 1/2 of a/b/c",
        "see https://example.com/some/deep/page for details",
        "mirror at //cdn.example.com/assets/app.js today",
        "the /etc directory and a lone / are not paths here",
        "a Windows path C:\\Users\\synthetic\\file.txt stays",
        "no slashes at all in this one",
        "a file URL file:///home/synthetic/work/notes.md stays whole",
        "cmd>/alpha/one.md is outside the stated prefix rule",
        "<file>/alpha/one.md</file> is outside the stated prefix rule",
        "read(/alpha/one.md) is outside the stated prefix rule",
    ]
    for sample in samples:
        tape = convert_lines([user_text(sample)])
        assert tape["events"][0]["text"] == sample, sample
        assert tape["run"]["redacted_values"] == 0, sample


def test_whole_value_paths_match_the_original_ft8_behaviour():
    tape = convert_lines([tool_use({"path": "/home/synthetic-user/work/notes.md"})])
    assert tape["events"][0]["input"] == {"path": "notes.md"}
    assert tape["run"]["redacted_values"] == 1


def test_a_tool_result_in_block_form_is_redacted():
    tape = convert_lines([tool_result(
        [{"type": "text", "text": f"contents of {SECRET} follow"}])])
    assert tape["events"][0]["content"] == "contents of secret-plan.md follow"
    assert tape["run"]["redacted_values"] == 1


def test_tool_input_keys_are_redacted_like_values():
    tape = convert_lines([tool_use({"mapping": {SECRET: "v1", "plain": SECRET}})])
    assert tape["events"][0]["input"] == {
        "mapping": {"secret-plan.md": "v1", "plain": "secret-plan.md"}}
    assert tape["run"]["redacted_values"] == 2
    deep = convert_lines([tool_use({"outer": [{"deep": {SECRET: True}}]})])
    assert deep["events"][0]["input"] == {"outer": [{"deep": {"secret-plan.md": True}}]}
    assert deep["run"]["redacted_values"] == 1


def test_keys_that_redact_to_the_same_name_merge():
    tape = convert_lines([tool_use(
        {"/home/synthetic-user/a/same.md": 1,
         "/home/synthetic-user/b/same.md": 2})])
    assert tape["events"][0]["input"] == {"same.md": 2}
    assert tape["run"]["redacted_values"] == 2


def test_a_non_string_text_value_is_carried_not_fatal():
    for value in (123, ["/a/b"], {"x": 1}, True):
        tape = convert_lines([line("assistant", [{"type": "text", "text": value}])])
        assert tape["events"][0]["text"] == value


def test_paths_glued_into_one_token_are_each_redacted():
    cases = [
        ("see /alpha/one.md,/beta/two.md end", "see one.md,two.md end", 2),
        ("paths /alpha/one.md:/beta/two.md end", "paths one.md:two.md end", 2),
        ("set a/b=/alpha/one.md end", "set a/b=one.md end", 1),
        ("open /alpha/one.md?next=http://example.com/x end",
         "open one.md?next=http://example.com/x end", 1),
        ("[report](/alpha/one.md) linked", "[report](one.md) linked", 1),
        ("click [the plan](/alpha/one.md) now",
         "click [the plan](one.md) now", 1),
        ("run x='/alpha/one.md' now", "run x='one.md' now", 1),
        ("file:=/alpha/one.md", "file:=one.md", 1),
        ("a::/alpha/one.md", "a::one.md", 1),
    ]
    for given, expected, count in cases:
        tape = convert_lines([user_text(given)])
        assert tape["events"][0]["text"] == expected, given
        assert tape["run"]["redacted_values"] == count, given


def test_the_fixture_tape_is_unchanged():
    expected = {
        "tape_version": "1.0",
        "run": {
            "harness": "claude-code",
            "session_id": "synthetic-session-001",
            "source_format": "claude-code-jsonl",
            "skipped_lines": {"system": 1},
            "redacted_values": 0,
            "totals": {"input_tokens": 150, "output_tokens": 50,
                       "cache_creation_input_tokens": 0,
                       "cache_read_input_tokens": 0,
                       "tool_calls": 1, "messages": 3},
            "started_at": "2026-01-01T09:00:00.000Z",
        },
        "events": [
            {"seq": 0, "kind": "message", "sidechain": False,
             "timestamp": "2026-01-01T09:00:00.000Z", "role": "user",
             "text": "Synthetic prompt, fix the widget counter."},
            {"seq": 1, "kind": "message", "sidechain": False,
             "timestamp": "2026-01-01T09:00:05.000Z", "role": "assistant",
             "text": "Synthetic reply, reading the counter first."},
            {"seq": 2, "kind": "tool_call", "sidechain": False,
             "timestamp": "2026-01-01T09:00:05.000Z",
             "call_id": "toolu_synth_1", "tool": "Read",
             "input": {"path": "widget.py"}},
            {"seq": 3, "kind": "tool_result", "sidechain": False,
             "timestamp": "2026-01-01T09:00:06.000Z",
             "call_id": "toolu_synth_1", "is_error": False,
             "content": "counter = 0"},
            {"seq": 4, "kind": "message", "sidechain": True,
             "timestamp": "2026-01-01T09:01:00.000Z", "role": "assistant",
             "text": "Synthetic sidechain note, checking a hunch."},
        ],
        "outcome": {"status": "unknown", "basis": "unresolved-ending"},
    }
    assert convert_file(FIXTURE) == expected
