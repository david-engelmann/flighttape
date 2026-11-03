//! The JSON reader (FT-51). Accepts what Python's json
//! accepts for a tape file and refuses the rest in the
//! loader's own terms. The parser is iterative, so a
//! deeply nested file is counted whole, never crashed.

use crate::value::{Text, Value};

#[derive(Clone, Debug, PartialEq)]
pub enum ParseError {
    /// Any malformed input; the loader calls the file
    /// not valid JSON.
    Invalid,
    /// An integer literal past Python's 4300-digit
    /// conversion limit; the loader names it apart.
    TooLarge,
}

const INT_DIGIT_LIMIT: usize = 4300;

enum Frame {
    Array(Vec<Value>),
    Object(Vec<(Text, Value)>, Option<Text>),
}

pub fn parse(text: &str) -> Result<Value, ParseError> {
    let bytes = text.as_bytes();
    let mut pos = 0usize;
    let mut stack: Vec<Frame> = Vec::new();
    let mut finished: Option<Value> = None;

    macro_rules! fail {
        () => {
            return Err(ParseError::Invalid)
        };
    }

    fn skip_ws(bytes: &[u8], pos: &mut usize) {
        while *pos < bytes.len()
            && matches!(bytes[*pos], b' ' | b'\t' | b'\n' | b'\r')
        {
            *pos += 1;
        }
    }

    // The machine reads one value at a time; containers
    // push frames, completed values attach upward.
    let mut expecting_value = true;
    loop {
        skip_ws(bytes, &mut pos);
        if finished.is_some() {
            if pos != bytes.len() {
                fail!();
            }
            return Ok(finished.take().unwrap());
        }
        if pos >= bytes.len() {
            fail!();
        }
        if expecting_value {
            match bytes[pos] {
                b'{' => {
                    pos += 1;
                    skip_ws(bytes, &mut pos);
                    if pos < bytes.len() && bytes[pos] == b'}' {
                        pos += 1;
                        attach(&mut stack, &mut finished, Value::Object(Vec::new()));
                    } else {
                        stack.push(Frame::Object(Vec::new(), None));
                        read_key(bytes, &mut pos, &mut stack)?;
                    }
                }
                b'[' => {
                    pos += 1;
                    skip_ws(bytes, &mut pos);
                    if pos < bytes.len() && bytes[pos] == b']' {
                        pos += 1;
                        attach(&mut stack, &mut finished, Value::Array(Vec::new()));
                    } else {
                        stack.push(Frame::Array(Vec::new()));
                    }
                }
                b'"' => {
                    let value = parse_string(bytes, &mut pos)?;
                    attach(&mut stack, &mut finished, Value::Str(value));
                }
                b't' => {
                    literal(bytes, &mut pos, "true")?;
                    attach(&mut stack, &mut finished, Value::Bool(true));
                }
                b'f' => {
                    literal(bytes, &mut pos, "false")?;
                    attach(&mut stack, &mut finished, Value::Bool(false));
                }
                b'n' => {
                    literal(bytes, &mut pos, "null")?;
                    attach(&mut stack, &mut finished, Value::Null);
                }
                b'N' => {
                    literal(bytes, &mut pos, "NaN")?;
                    attach(&mut stack, &mut finished, Value::Float(f64::NAN));
                }
                b'I' => {
                    literal(bytes, &mut pos, "Infinity")?;
                    attach(&mut stack, &mut finished, Value::Float(f64::INFINITY));
                }
                b'-' | b'0'..=b'9' => {
                    let value = parse_number(bytes, &mut pos)?;
                    attach(&mut stack, &mut finished, value);
                }
                _ => fail!(),
            }
            expecting_value = false;
        } else {
            // After a value inside a container: comma or close.
            match stack.last_mut() {
                None => {
                    // A finished top-level value is handled
                    // at the loop head.
                }
                Some(Frame::Array(_)) => match bytes[pos] {
                    b',' => {
                        pos += 1;
                        expecting_value = true;
                    }
                    b']' => {
                        pos += 1;
                        if let Some(Frame::Array(items)) = stack.pop() {
                            attach(&mut stack, &mut finished, Value::Array(items));
                        }
                    }
                    _ => fail!(),
                },
                Some(Frame::Object(_, _)) => match bytes[pos] {
                    b',' => {
                        pos += 1;
                        read_key(bytes, &mut pos, &mut stack)?;
                        expecting_value = true;
                    }
                    b'}' => {
                        pos += 1;
                        if let Some(Frame::Object(pairs, _)) = stack.pop() {
                            attach(&mut stack, &mut finished, Value::Object(pairs));
                        }
                    }
                    _ => fail!(),
                },
            }
        }
    }
}

fn attach(stack: &mut Vec<Frame>, finished: &mut Option<Value>, value: Value) {
    match stack.last_mut() {
        None => *finished = Some(value),
        Some(Frame::Array(items)) => items.push(value),
        Some(Frame::Object(pairs, pending)) => {
            let key = pending.take().expect("object value follows its key");
            match pairs.iter_mut().find(|(name, _)| *name == key) {
                Some((_, existing)) => *existing = value,
                None => pairs.push((key, value)),
            }
        }
    }
}

fn read_key(
    bytes: &[u8],
    pos: &mut usize,
    stack: &mut Vec<Frame>,
) -> Result<(), ParseError> {
    while *pos < bytes.len()
        && matches!(bytes[*pos], b' ' | b'\t' | b'\n' | b'\r')
    {
        *pos += 1;
    }
    if *pos >= bytes.len() || bytes[*pos] != b'"' {
        return Err(ParseError::Invalid);
    }
    let key = parse_string(bytes, pos)?;
    while *pos < bytes.len()
        && matches!(bytes[*pos], b' ' | b'\t' | b'\n' | b'\r')
    {
        *pos += 1;
    }
    if *pos >= bytes.len() || bytes[*pos] != b':' {
        return Err(ParseError::Invalid);
    }
    *pos += 1;
    if let Some(Frame::Object(_, pending)) = stack.last_mut() {
        *pending = Some(key);
    }
    Ok(())
}

fn literal(bytes: &[u8], pos: &mut usize, word: &str) -> Result<(), ParseError> {
    if bytes.len() - *pos >= word.len()
        && &bytes[*pos..*pos + word.len()] == word.as_bytes()
    {
        *pos += word.len();
        Ok(())
    } else {
        Err(ParseError::Invalid)
    }
}

fn parse_string(bytes: &[u8], pos: &mut usize) -> Result<Text, ParseError> {
    // bytes[*pos] is the opening quote. The input is a
    // valid UTF-8 str; raw characters pass through as
    // their code units, control characters are refused
    // raw, as Python's strict reader refuses them.
    debug_assert_eq!(bytes[*pos], b'"');
    *pos += 1;
    let mut units: Vec<u16> = Vec::new();
    loop {
        if *pos >= bytes.len() {
            return Err(ParseError::Invalid);
        }
        let byte = bytes[*pos];
        match byte {
            b'"' => {
                *pos += 1;
                return Ok(Text::from_units(units));
            }
            b'\\' => {
                *pos += 1;
                if *pos >= bytes.len() {
                    return Err(ParseError::Invalid);
                }
                let escape = bytes[*pos];
                *pos += 1;
                match escape {
                    b'"' => units.push(b'"' as u16),
                    b'\\' => units.push(b'\\' as u16),
                    b'/' => units.push(b'/' as u16),
                    b'b' => units.push(0x08),
                    b'f' => units.push(0x0C),
                    b'n' => units.push(b'\n' as u16),
                    b'r' => units.push(b'\r' as u16),
                    b't' => units.push(b'\t' as u16),
                    b'u' => {
                        let first = hex4(bytes, pos)?;
                        if (0xD800..0xDC00).contains(&first) {
                            // A high surrogate: combine a
                            // following low surrogate when
                            // one stands there, else keep
                            // the unit unpaired, as Python
                            // keeps it.
                            let saved = *pos;
                            if *pos + 1 < bytes.len()
                                && bytes[*pos] == b'\\'
                                && bytes[*pos + 1] == b'u'
                            {
                                *pos += 2;
                                let second = hex4(bytes, pos)?;
                                if (0xDC00..0xE000).contains(&second) {
                                    let scalar = 0x10000u32
                                        + (((first as u32 - 0xD800) << 10)
                                            | (second as u32 - 0xDC00));
                                    let ch = char::from_u32(scalar)
                                        .ok_or(ParseError::Invalid)?;
                                    let mut buffer = [0u16; 2];
                                    let encoded: &[u16] =
                                        ch.encode_utf16(&mut buffer);
                                    units.extend_from_slice(encoded);
                                } else {
                                    units.push(first);
                                    units.push(second);
                                }
                            } else {
                                *pos = saved;
                                units.push(first);
                            }
                        } else {
                            units.push(first);
                        }
                    }
                    _ => return Err(ParseError::Invalid),
                }
            }
            0x00..=0x1F => return Err(ParseError::Invalid),
            _ => {
                // One UTF-8 encoded character, as units.
                let text = std::str::from_utf8(&bytes[*pos..])
                    .map_err(|_| ParseError::Invalid)?;
                let ch = text.chars().next().ok_or(ParseError::Invalid)?;
                let mut buffer = [0u16; 2];
                let encoded: &[u16] = ch.encode_utf16(&mut buffer);
                units.extend_from_slice(encoded);
                *pos += ch.len_utf8();
            }
        }
    }
}

fn hex4(bytes: &[u8], pos: &mut usize) -> Result<u16, ParseError> {
    if bytes.len() - *pos < 4 {
        return Err(ParseError::Invalid);
    }
    let mut value: u16 = 0;
    for offset in 0..4 {
        let digit = match bytes[*pos + offset] {
            b @ b'0'..=b'9' => (b - b'0') as u16,
            b @ b'a'..=b'f' => (b - b'a' + 10) as u16,
            b @ b'A'..=b'F' => (b - b'A' + 10) as u16,
            _ => return Err(ParseError::Invalid),
        };
        value = value * 16 + digit;
    }
    *pos += 4;
    Ok(value)
}

fn parse_number(bytes: &[u8], pos: &mut usize) -> Result<Value, ParseError> {
    let start = *pos;
    if bytes[*pos] == b'-' {
        *pos += 1;
        if *pos < bytes.len() && bytes[*pos] == b'I' {
            literal(bytes, pos, "Infinity")?;
            return Ok(Value::Float(f64::NEG_INFINITY));
        }
    }
    // Integer part: 0 or a nonzero digit run.
    if *pos >= bytes.len() {
        return Err(ParseError::Invalid);
    }
    match bytes[*pos] {
        b'0' => *pos += 1,
        b'1'..=b'9' => {
            while *pos < bytes.len() && bytes[*pos].is_ascii_digit() {
                *pos += 1;
            }
        }
        _ => return Err(ParseError::Invalid),
    }
    let mut is_float = false;
    if *pos < bytes.len() && bytes[*pos] == b'.' {
        is_float = true;
        *pos += 1;
        if *pos >= bytes.len() || !bytes[*pos].is_ascii_digit() {
            return Err(ParseError::Invalid);
        }
        while *pos < bytes.len() && bytes[*pos].is_ascii_digit() {
            *pos += 1;
        }
    }
    if *pos < bytes.len() && matches!(bytes[*pos], b'e' | b'E') {
        is_float = true;
        *pos += 1;
        if *pos < bytes.len() && matches!(bytes[*pos], b'+' | b'-') {
            *pos += 1;
        }
        if *pos >= bytes.len() || !bytes[*pos].is_ascii_digit() {
            return Err(ParseError::Invalid);
        }
        while *pos < bytes.len() && bytes[*pos].is_ascii_digit() {
            *pos += 1;
        }
    }
    let text = std::str::from_utf8(&bytes[start..*pos])
        .map_err(|_| ParseError::Invalid)?;
    if !is_float {
        let digits: String = if text == "-0" {
            "0".to_string()
        } else {
            text.to_string()
        };
        let count = digits.trim_start_matches('-').len();
        if count > INT_DIGIT_LIMIT {
            return Err(ParseError::TooLarge);
        }
        Ok(Value::Int(digits))
    } else {
        text.parse::<f64>()
            .map(Value::Float)
            .map_err(|_| ParseError::Invalid)
    }
}

/// Render a value as JSON text with Python's
/// json.dumps defaults for layout: separators ", "
/// and ": " when indent is absent is not what dumps
/// does; flighttape writes with its own helpers. This
/// compact writer serves the canonical-adjacent needs
/// inside the crate; user-facing writers live in the
/// command modules.
pub fn write_compact(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Int(digits) => out.push_str(digits),
        Value::Float(number) => out.push_str(&crate::identity::render_number(*number)),
        Value::Str(text) => {
            let mut units: Vec<u16> = Vec::new();
            text.render_json(&mut units);
            match String::from_utf16(&units) {
                Ok(rendered) => out.push_str(&rendered),
                Err(_) => out.push_str("null"),
            }
        }
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_compact(item, out);
            }
            out.push(']');
        }
        Value::Object(pairs) => {
            out.push('{');
            for (index, (key, item)) in pairs.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_compact(&Value::Str(key.clone()), out);
                out.push(':');
                write_compact(item, out);
            }
            out.push('}');
        }
    }
}
