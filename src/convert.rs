//! Convert a Claude Code session JSONL file into a
//! flight tape. Mapping, per FT-6 and FT-7. Lines the
//! converter does not turn into events are counted in
//! run.skipped_lines by reason. A line from a second
//! session is counted and skipped, never merged.
//! Malformed lines are counted, never fatal. Absolute
//! paths inside tool inputs and text are reduced to
//! their final component and counted in
//! run.redacted_values (FT-8, FT-22 to FT-24). The
//! outcome is determined from the events, never a
//! constant. The converter never raises on its input
//! (FT-18): each line converts whole or is counted
//! whole.

use std::collections::HashMap;

use crate::outcome::determine_outcome;
use crate::value::{int_add, object, py_str, truthy, Text, Value};
use crate::TAPE_VERSION;

pub const SOURCE_FORMAT: &str = "claude-code-jsonl";
pub const TOKEN_KEYS: [&str; 4] = [
    "input_tokens",
    "output_tokens",
    "cache_creation_input_tokens",
    "cache_read_input_tokens",
];
pub const TOTAL_KEYS: [&str; 6] = [
    "input_tokens",
    "output_tokens",
    "cache_creation_input_tokens",
    "cache_read_input_tokens",
    "tool_calls",
    "messages",
];

/// Python's whitespace, the set its token split and
/// strip both use.
fn is_space(scalar: u32) -> bool {
    matches!(scalar,
        0x09..=0x0D | 0x1C..=0x1F | 0x20 | 0x85 | 0xA0 | 0x1680
        | 0x2000..=0x200A | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000)
}

/// The scalars of a text: surrogate pairs combined,
/// an unpaired unit standing as its own value.
fn scalars(text: &Text) -> Vec<u32> {
    let units = text.units();
    let mut out = Vec::with_capacity(units.len());
    let mut index = 0;
    while index < units.len() {
        let unit = units[index];
        if (0xD800..0xDC00).contains(&unit)
            && index + 1 < units.len()
            && (0xDC00..0xE000).contains(&units[index + 1])
        {
            out.push(
                0x10000 + (((unit as u32 - 0xD800) << 10) | (units[index + 1] as u32 - 0xDC00)),
            );
            index += 2;
        } else {
            out.push(unit as u32);
            index += 1;
        }
    }
    out
}

fn text_from_scalars(scalars: &[u32]) -> Text {
    let mut units: Vec<u16> = Vec::with_capacity(scalars.len());
    for scalar in scalars {
        if *scalar < 0x10000 {
            units.push(*scalar as u16);
        } else {
            let adjusted = scalar - 0x10000;
            units.push(0xD800 + (adjusted >> 10) as u16);
            units.push(0xDC00 + (adjusted & 0x3FF) as u16);
        }
    }
    Text::from_units(units)
}

const PREFIX_CHARS: &[u32] = &[
    b'"' as u32,
    b'\'' as u32,
    b'`' as u32,
    b'(' as u32,
    b'[' as u32,
    b'{' as u32,
    b'<' as u32,
];
const TRAILING_PUNCT: &[u32] = &[
    b'.' as u32,
    b',' as u32,
    b';' as u32,
    b':' as u32,
    b'!' as u32,
    b'?' as u32,
    b')' as u32,
    b']' as u32,
    b'}' as u32,
    b'"' as u32,
    b'\'' as u32,
    b'`' as u32,
];
const PATH_STOP: &[u32] = &[
    b',' as u32,
    b';' as u32,
    b'?' as u32,
    b'#' as u32,
    b'&' as u32,
];

fn stem_matches(stem: &[u32]) -> bool {
    // [A-Za-z0-9_.-]*[=:]+\z as a full match.
    let mut index = stem.len();
    let mut separators = 0;
    while index > 0 && (stem[index - 1] == b'=' as u32 || stem[index - 1] == b':' as u32) {
        index -= 1;
        separators += 1;
    }
    if separators == 0 {
        return false;
    }
    stem[..index].iter().all(|ch| {
        matches!(ch, 48..=57 | 65..=90 | 97..=122)
            || *ch == b'_' as u32
            || *ch == b'.' as u32
            || *ch == b'-' as u32
    })
}

/// Judge the local prefix before a slash, per FT-23.
fn qualifies(local: &[u32]) -> bool {
    if local.is_empty() {
        return true;
    }
    let mut run = local.len();
    while run > 0 && PREFIX_CHARS.contains(&local[run - 1]) {
        run -= 1;
    }
    let stem = &local[..run];
    if run < local.len() {
        return stem.is_empty()
            || stem.ends_with(&[b'=' as u32])
            || stem.ends_with(&[b':' as u32])
            || stem.ends_with(&[b']' as u32])
            || stem_matches(stem);
    }
    stem_matches(local)
}

/// Replace every absolute path in one token, per
/// FT-23. The counter counts the replacements.
fn redact_token(token: &[u32], counter: &mut u64) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    let mut seg = 0usize;
    let mut origin = 0usize;
    let mut i = 0usize;
    let size = token.len();
    while i < size {
        let ch = token[i];
        if ch != b'/' as u32 {
            if PATH_STOP.contains(&ch) {
                origin = i + 1;
            }
            i += 1;
            continue;
        }
        if token[i..].starts_with(&[b'/' as u32, b'/' as u32]) {
            i += 2;
            origin = i;
            continue;
        }
        if i > 0 && token[i - 1] == b'/' as u32 {
            i += 1;
            origin = i;
            continue;
        }
        if !qualifies(&token[origin..i]) {
            i += 1;
            origin = i;
            continue;
        }
        let mut j = i;
        while j < size && !PATH_STOP.contains(&token[j]) {
            if token[j] == b':' as u32 && j + 1 < size && token[j + 1] == b'/' as u32 {
                break;
            }
            j += 1;
        }
        let run = &token[i..j];
        let mut end = run.len();
        while end > 0 && TRAILING_PUNCT.contains(&run[end - 1]) {
            end -= 1;
        }
        let head = &run[..end];
        let parts: Vec<&[u32]> = head
            .split(|ch| *ch == b'/' as u32)
            .filter(|part| !part.is_empty())
            .collect();
        let has_scheme = head
            .windows(3)
            .any(|window| window == [b':' as u32, b'/' as u32, b'/' as u32]);
        if has_scheme || parts.len() < 2 {
            i += 1;
            origin = i;
            continue;
        }
        out.extend_from_slice(&token[seg..i]);
        out.extend_from_slice(parts[parts.len() - 1]);
        out.extend_from_slice(&run[end..]);
        *counter += 1;
        i = j;
        seg = j;
        origin = j;
    }
    out.extend_from_slice(&token[seg..]);
    out
}

/// Redact every absolute path in a text string
/// (FT-22). A value that is not a string is routed
/// through sanitize, so the string leaves inside a
/// list or an object are redacted the same way.
pub fn redact_text(value: &Value, counter: &mut u64) -> Value {
    let text = match value {
        Value::Str(text) => text,
        _ => return sanitize(value, counter),
    };
    let chars = scalars(text);
    let mut out: Vec<u32> = Vec::with_capacity(chars.len());
    let mut index = 0;
    while index < chars.len() {
        let space = is_space(chars[index]);
        let start = index;
        while index < chars.len() && is_space(chars[index]) == space {
            index += 1;
        }
        let piece = &chars[start..index];
        if space {
            out.extend_from_slice(piece);
        } else {
            out.extend_from_slice(&redact_token(piece, counter));
        }
    }
    Value::Str(text_from_scalars(&out))
}

/// How many absolute paths a string holds (FT-45):
/// exactly the number of replacements redact_text
/// would make in it.
pub fn paths_in(text: &Text) -> u64 {
    let mut counter = 0u64;
    let _ = redact_text(&Value::Str(text.clone()), &mut counter);
    counter
}

/// Redact a value whole: every string leaf and every
/// object key, per FT-22. Iterative, so no depth a
/// parsed input can hold exhausts a call stack.
pub fn sanitize(value: &Value, counter: &mut u64) -> Value {
    enum Task<'a> {
        Build(&'a Value),
        AssembleObject(Vec<Text>, usize),
        AssembleArray(usize),
    }
    let mut results: Vec<Value> = Vec::new();
    let mut tasks: Vec<Task> = vec![Task::Build(value)];
    while let Some(task) = tasks.pop() {
        match task {
            Task::Build(current) => match current {
                Value::Str(_) => {
                    results.push(redact_text(current, counter));
                }
                Value::Array(items) => {
                    tasks.push(Task::AssembleArray(items.len()));
                    for item in items.iter().rev() {
                        tasks.push(Task::Build(item));
                    }
                }
                Value::Object(pairs) => {
                    let mut keys: Vec<Text> = Vec::with_capacity(pairs.len());
                    for (key, _) in pairs {
                        let redacted = redact_text(&Value::Str(key.clone()), counter);
                        match redacted {
                            Value::Str(text) => keys.push(text),
                            _ => keys.push(key.clone()),
                        }
                    }
                    tasks.push(Task::AssembleObject(keys, pairs.len()));
                    for (_, item) in pairs.iter().rev() {
                        tasks.push(Task::Build(item));
                    }
                }
                _ => results.push(current.clone()),
            },
            Task::AssembleArray(count) => {
                let mut items = results.split_off(results.len() - count);
                let mut assembled = Vec::new();
                assembled.append(&mut items);
                results.push(Value::Array(assembled));
            }
            Task::AssembleObject(keys, count) => {
                let values = results.split_off(results.len() - count);
                let mut pairs: Vec<(Text, Value)> = Vec::new();
                // A position index keeps the merge
                // linear; a search per key made wide
                // objects quadratic.
                let mut positions: HashMap<Text, usize> = HashMap::new();
                for (key, item) in keys.into_iter().zip(values) {
                    match positions.get(&key) {
                        Some(&position) => pairs[position].1 = item,
                        None => {
                            positions.insert(key.clone(), pairs.len());
                            pairs.push((key, item));
                        }
                    }
                }
                results.push(Value::Object(pairs));
            }
        }
    }
    results.pop().expect("sanitize yields one value")
}

fn result_text(content: Option<&Value>) -> Text {
    match content {
        Some(Value::Str(text)) => text.clone(),
        Some(Value::Array(blocks)) => {
            let mut parts: Vec<Text> = Vec::new();
            for block in blocks {
                if block.as_object().is_some()
                    && block.get("type") == Some(&Value::Str(Text::from_str("text")))
                {
                    let part = match block.get("text") {
                        None => Text::from_str(""),
                        Some(value) => py_str(value),
                    };
                    parts.push(part);
                }
            }
            let mut joined: Vec<u16> = Vec::new();
            for (index, part) in parts.iter().enumerate() {
                if index > 0 {
                    joined.push(b'\n' as u16);
                }
                joined.extend_from_slice(part.units());
            }
            Text::from_units(joined)
        }
        _ => Text::from_str(""),
    }
}

/// An ordered count map for skipped lines.
struct Counts {
    entries: Vec<(Text, u64)>,
}

impl Counts {
    fn new() -> Counts {
        Counts {
            entries: Vec::new(),
        }
    }

    fn bump(&mut self, key: Text) {
        match self.entries.iter_mut().find(|(name, _)| *name == key) {
            Some((_, count)) => *count += 1,
            None => self.entries.push((key, 1)),
        }
    }

    fn absorb(&mut self, other: Counts) {
        for (key, count) in other.entries {
            match self.entries.iter_mut().find(|(name, _)| *name == key) {
                Some((_, existing)) => *existing += count,
                None => self.entries.push((key, count)),
            }
        }
    }

    fn to_value(&self) -> Value {
        Value::Object(
            self.entries
                .iter()
                .map(|(key, count)| (key.clone(), Value::Int(count.to_string())))
                .collect(),
        )
    }
}

/// Totals over the six standing keys, held as exact
/// digit strings.
struct Totals {
    entries: Vec<(String, String)>,
}

impl Totals {
    fn new() -> Totals {
        Totals {
            entries: TOTAL_KEYS
                .iter()
                .map(|key| (key.to_string(), "0".to_string()))
                .collect(),
        }
    }

    fn add(&mut self, key: &str, digits: &str) {
        let entry = self
            .entries
            .iter_mut()
            .find(|(name, _)| name == key)
            .expect("totals keys are the standing six");
        entry.1 = int_add(&entry.1, digits);
    }

    fn add_count(&mut self, key: &str) {
        self.add(key, "1");
    }

    fn absorb(&mut self, other: &Totals) {
        for (key, digits) in &other.entries {
            self.add(key, digits);
        }
    }

    fn to_value(&self) -> Value {
        Value::Object(
            self.entries
                .iter()
                .map(|(key, digits)| (Text::from_str(key), Value::Int(digits.clone())))
                .collect(),
        )
    }
}

struct LineResult {
    events: Vec<Value>,
    skipped: Counts,
    totals: Totals,
    redactions: u64,
    adopt_session: Option<Value>,
    started_at: Option<Value>,
}

fn convert_line(line: &Value, session_id: Option<&Value>, base_seq: usize) -> LineResult {
    let mut events: Vec<Value> = Vec::new();
    let mut skipped = Counts::new();
    let mut totals = Totals::new();
    let mut redactions: u64 = 0;
    let mut adopt_session: Option<Value> = None;
    let mut started_at: Option<Value> = None;

    let sidechain = truthy(line.get("isSidechain").unwrap_or(&Value::Bool(false)));
    let timestamp = line.get("timestamp").cloned();

    macro_rules! emit {
        ($kind:expr, $fields:expr) => {{
            let mut pairs: Vec<(Text, Value)> = vec![
                (
                    Text::from_str("seq"),
                    Value::Int((base_seq + events.len()).to_string()),
                ),
                (Text::from_str("kind"), Value::Str(Text::from_str($kind))),
                (Text::from_str("sidechain"), Value::Bool(sidechain)),
            ];
            if let Some(stamp) = &timestamp {
                if truthy(stamp) {
                    pairs.push((Text::from_str("timestamp"), stamp.clone()));
                }
            }
            pairs.extend($fields);
            events.push(Value::Object(pairs));
        }};
    }

    if let Some(seen) = line.get("sessionId") {
        if truthy(seen) {
            match session_id {
                None => adopt_session = Some(seen.clone()),
                Some(current) => {
                    if seen != current {
                        skipped.bump(Text::from_str("foreign-session"));
                        return LineResult {
                            events: Vec::new(),
                            skipped,
                            totals: Totals::new(),
                            redactions: 0,
                            adopt_session: None,
                            started_at: None,
                        };
                    }
                }
            }
        }
    }
    if let Some(stamp) = &timestamp {
        if truthy(stamp) {
            started_at = Some(stamp.clone());
        }
    }
    let line_type = line.get("type").cloned();
    let is_user = line_type == Some(Value::Str(Text::from_str("user")));
    let is_assistant = line_type == Some(Value::Str(Text::from_str("assistant")));
    if !is_user && !is_assistant {
        match &line_type {
            Some(Value::Str(text)) => {
                if text.units().is_empty() {
                    skipped.bump(Text::from_str("missing-type"));
                } else {
                    skipped.bump(text.clone());
                }
            }
            Some(_) => skipped.bump(Text::from_str("malformed:type")),
            None => skipped.bump(Text::from_str("missing-type")),
        }
        return LineResult {
            events,
            skipped,
            totals,
            redactions,
            adopt_session,
            started_at,
        };
    }
    let type_name = if is_user { "user" } else { "assistant" };
    let message = match line.get("message") {
        Some(message @ Value::Object(_)) => message.clone(),
        _ => {
            skipped.bump(Text::from_str("malformed:message"));
            return LineResult {
                events,
                skipped,
                totals,
                redactions,
                adopt_session,
                started_at,
            };
        }
    };
    let role = match message.get("role") {
        Some(role) if truthy(role) => role.clone(),
        _ => Value::Str(Text::from_str(type_name)),
    };
    match message.get("usage") {
        Some(usage @ Value::Object(_)) => {
            for key in TOKEN_KEYS {
                if let Some(Value::Int(digits)) = usage.get(key) {
                    totals.add(key, digits);
                }
            }
        }
        Some(_) => skipped.bump(Text::from_str("malformed:usage")),
        None => {}
    }
    let mut content: Option<Value> = message.get("content").cloned();
    if let Some(Value::Str(text)) = &content {
        content = Some(if text.units().is_empty() {
            Value::Array(Vec::new())
        } else {
            Value::Array(vec![object(vec![
                ("type", Value::Str(Text::from_str("text"))),
                ("text", Value::Str(text.clone())),
            ])])
        });
    }
    let blocks = match &content {
        Some(Value::Array(blocks)) if !blocks.is_empty() => blocks.clone(),
        _ => {
            skipped.bump(Text::from_str(&format!("contentless:{}", type_name)));
            return LineResult {
                events,
                skipped,
                totals,
                redactions,
                adopt_session,
                started_at,
            };
        }
    };
    let mut made = 0usize;
    for block in &blocks {
        if block.as_object().is_none() {
            skipped.bump(Text::from_str("block:non-object"));
            continue;
        }
        let block_type = block.get("type").cloned();
        if block_type == Some(Value::Str(Text::from_str("text"))) {
            if let Some(text) = block.get("text") {
                if truthy(text) {
                    let redacted = redact_text(text, &mut redactions);
                    emit!(
                        "message",
                        vec![
                            (Text::from_str("role"), role.clone()),
                            (Text::from_str("text"), redacted),
                        ]
                    );
                    totals.add_count("messages");
                    made += 1;
                }
            }
        } else if block_type == Some(Value::Str(Text::from_str("tool_use"))) {
            let input = match block.get("input") {
                Some(value) if truthy(value) => value.clone(),
                _ => Value::Object(Vec::new()),
            };
            let sanitized = sanitize(&input, &mut redactions);
            emit!(
                "tool_call",
                vec![
                    (
                        Text::from_str("call_id"),
                        block.get("id").cloned().unwrap_or(Value::Null),
                    ),
                    (
                        Text::from_str("tool"),
                        block.get("name").cloned().unwrap_or(Value::Null),
                    ),
                    (Text::from_str("input"), sanitized),
                ]
            );
            totals.add_count("tool_calls");
            made += 1;
        } else if block_type == Some(Value::Str(Text::from_str("tool_result"))) {
            let text = result_text(block.get("content"));
            let redacted = redact_text(&Value::Str(text), &mut redactions);
            emit!(
                "tool_result",
                vec![
                    (
                        Text::from_str("call_id"),
                        block.get("tool_use_id").cloned().unwrap_or(Value::Null),
                    ),
                    (
                        Text::from_str("is_error"),
                        Value::Bool(truthy(block.get("is_error").unwrap_or(&Value::Bool(false)),)),
                    ),
                    (Text::from_str("content"), redacted),
                ]
            );
            made += 1;
        } else {
            let key = match &block_type {
                Some(value) => py_str(value),
                None => Text::from_str("None"),
            };
            let mut bump_key = Text::from_str("block:");
            let mut units = bump_key.units().to_vec();
            units.extend_from_slice(key.units());
            bump_key = Text::from_units(units);
            skipped.bump(bump_key);
        }
    }
    if made == 0 {
        skipped.bump(Text::from_str(&format!("contentless:{}", type_name)));
    }
    LineResult {
        events,
        skipped,
        totals,
        redactions,
        adopt_session,
        started_at,
    }
}

/// Convert an iterable of session lines into a tape.
/// An entry of None marks a line that was not valid
/// UTF-8, counted as undecodable. No entry, however
/// malformed, raises (FT-18).
pub fn convert_lines(lines: Vec<Option<String>>) -> Value {
    let mut events: Vec<Value> = Vec::new();
    let mut skipped = Counts::new();
    let mut totals = Totals::new();
    let mut redactions: u64 = 0;
    let mut session_id: Option<Value> = None;
    let mut started_at: Option<Value> = None;

    for raw in &lines {
        let raw = match raw {
            None => {
                skipped.bump(Text::from_str("undecodable"));
                continue;
            }
            Some(raw) => raw,
        };
        let trimmed = raw.trim_matches(|ch: char| is_space(ch as u32));
        if trimmed.is_empty() {
            continue;
        }
        let line = match crate::json::parse(trimmed) {
            Ok(line) => line,
            Err(_) => {
                skipped.bump(Text::from_str("unparseable"));
                continue;
            }
        };
        if line.as_object().is_none() {
            skipped.bump(Text::from_str("non-object"));
            continue;
        }
        if crate::value::container_depth(&line) > 1000 {
            // The Python's parser and converter raise
            // RecursionError there and count the line
            // too-deep; the port counts whole at any
            // depth (FT-51) and refuses past the
            // canonical limit instead (FT-54).
            skipped.bump(Text::from_str("too-deep"));
            continue;
        }
        let result = convert_line(&line, session_id.as_ref(), events.len());
        events.extend(result.events);
        skipped.absorb(result.skipped);
        totals.absorb(&result.totals);
        redactions += result.redactions;
        if session_id.is_none() && result.adopt_session.is_some() {
            session_id = result.adopt_session;
        }
        if started_at.is_none() && result.started_at.is_some() {
            started_at = result.started_at;
        }
    }
    let session_value = match &session_id {
        Some(value) if truthy(value) => value.clone(),
        _ => Value::Str(Text::from_str("unknown")),
    };
    let mut run_pairs: Vec<(Text, Value)> = vec![
        (
            Text::from_str("harness"),
            Value::Str(Text::from_str("claude-code")),
        ),
        (Text::from_str("session_id"), session_value),
        (
            Text::from_str("source_format"),
            Value::Str(Text::from_str(SOURCE_FORMAT)),
        ),
        (Text::from_str("skipped_lines"), skipped.to_value()),
        (
            Text::from_str("redacted_values"),
            Value::Int(redactions.to_string()),
        ),
        (Text::from_str("totals"), totals.to_value()),
    ];
    if let Some(started) = &started_at {
        if truthy(started) {
            run_pairs.push((Text::from_str("started_at"), started.clone()));
        }
    }
    let events_value = Value::Array(events);
    let outcome = determine_outcome(&events_value);
    object(vec![
        ("tape_version", Value::Str(Text::from_str(TAPE_VERSION))),
        ("run", Value::Object(run_pairs)),
        ("events", events_value),
        ("outcome", outcome),
    ])
}

/// Convert a session file, reading it as bytes
/// (FT-18). A line that is not valid UTF-8 is handed
/// to convert_lines as None and counted as
/// undecodable, never raised.
pub fn convert_file(path: &std::path::Path) -> std::io::Result<Value> {
    let data = std::fs::read(path)?;
    let mut lines: Vec<Option<String>> = Vec::new();
    for raw in data.split(|byte| *byte == b'\n') {
        lines.push(String::from_utf8(raw.to_vec()).ok());
    }
    Ok(convert_lines(lines))
}
