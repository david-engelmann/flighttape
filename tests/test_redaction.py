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
    ]
    for sample in samples:
        tape = convert_lines([user_text(sample)])
        assert tape["events"][0]["text"] == sample, sample
        assert tape["run"]["redacted_values"] == 0, sample


def test_whole_value_paths_match_the_original_ft8_behaviour():
    tape = convert_lines([tool_use({"path": "/home/synthetic-user/work/notes.md"})])
    assert tape["events"][0]["input"] == {"path": "notes.md"}
    assert tape["run"]["redacted_values"] == 1


def test_the_fixture_tape_is_unchanged():
    tape = convert_file(FIXTURE)
    assert tape["run"]["redacted_values"] == 0
    assert len(tape["events"]) == 5
    assert tape["events"][0]["text"] == "Synthetic prompt, fix the widget counter."
