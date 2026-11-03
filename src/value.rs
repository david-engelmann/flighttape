//! The tape value type (FT-51, FT-53). Mirrors the values
//! Python's json produces: integers of any size held
//! exactly as their decimal digits, floats as f64, and
//! text as UTF-16 code units, the units the spec's own
//! rules are stated in.

use std::cmp::Ordering;

/// Text as UTF-16 code units. A string carrying an
/// unpaired surrogate is representable here, as it is
/// in Python; `as_str` returns None for such text.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Text(Vec<u16>);

impl Text {
    pub fn from_str(value: &str) -> Text {
        Text(value.encode_utf16().collect())
    }

    pub fn from_units(units: Vec<u16>) -> Text {
        Text(units)
    }

    pub fn units(&self) -> &[u16] {
        &self.0
    }

    /// The Rust string, when the units form valid text.
    pub fn as_str(&self) -> Option<String> {
        String::from_utf16(&self.0).ok()
    }

    /// True when no unit is an unpaired surrogate.
    pub fn is_well_formed(&self) -> bool {
        self.as_str().is_some()
    }

    /// Render as Python's json.dumps with
    /// ensure_ascii=False renders: the escapes for the
    /// quote, the backslash, and the control characters,
    /// everything else raw, unpaired surrogates passed
    /// through as units (the caller's final encoding
    /// refuses them, as canonical_bytes does).
    pub fn render_json(&self, out: &mut Vec<u16>) {
        out.push(b'"' as u16);
        for &unit in &self.0 {
            match unit {
                0x22 => out.extend_from_slice(&[b'\\' as u16, b'"' as u16]),
                0x5C => out.extend_from_slice(&[b'\\' as u16, b'\\' as u16]),
                0x08 => out.extend_from_slice(&[b'\\' as u16, b'b' as u16]),
                0x09 => out.extend_from_slice(&[b'\\' as u16, b't' as u16]),
                0x0A => out.extend_from_slice(&[b'\\' as u16, b'n' as u16]),
                0x0C => out.extend_from_slice(&[b'\\' as u16, b'f' as u16]),
                0x0D => out.extend_from_slice(&[b'\\' as u16, b'r' as u16]),
                0x00..=0x1F => {
                    let escape = format!("\\u{:04x}", unit);
                    out.extend(escape.encode_utf16());
                }
                _ => out.push(unit),
            }
        }
        out.push(b'"' as u16);
    }
}

impl PartialEq<&str> for Text {
    fn eq(&self, other: &&str) -> bool {
        match self.as_str() {
            Some(text) => text == *other,
            None => false,
        }
    }
}

impl PartialOrd for Text {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Text {
    /// UTF-16 code unit order, the canonical key order.
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

/// A tape value, mirroring Python's json values.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    /// Exact integer, canonical decimal digits (a
    /// leading minus allowed, no leading zeros, "0"
    /// for zero, never "-0").
    Int(String),
    Float(f64),
    Str(Text),
    Array(Vec<Value>),
    /// Document order. A repeated key keeps its first
    /// position with its last value, as a Python dict
    /// built by json.loads does.
    Object(Vec<(Text, Value)>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(pairs) => pairs
                .iter()
                .find(|(name, _)| name == &key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        match self {
            Value::Object(pairs) => pairs
                .iter_mut()
                .find(|(name, _)| name == &key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Vec<(Text, Value)>> {
        match self {
            Value::Object(pairs) => Some(pairs),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&Text> {
        match self {
            Value::Str(text) => Some(text),
            _ => None,
        }
    }

    /// True for an int or float that is not a bool,
    /// the validator's is_number.
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Int(_) | Value::Float(_))
    }

    /// The value as f64, as Python would compute with
    /// it: integers convert (huge ones to infinity),
    /// anything else is None.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(digits) => digits.parse::<f64>().ok(),
            Value::Float(value) => Some(*value),
            _ => None,
        }
    }

    /// The value as an exact integer's digits, when it
    /// is an integer or a float with an integral value
    /// Python would still call a float. Only Int
    /// carries digits; this returns them.
    pub fn int_digits(&self) -> Option<&str> {
        match self {
            Value::Int(digits) => Some(digits),
            _ => None,
        }
    }

    pub fn is_int(&self) -> bool {
        matches!(self, Value::Int(_))
    }
}

/// Compare two canonical digit strings as integers.
pub fn int_cmp(a: &str, b: &str) -> Ordering {
    let (a_neg, a_digits) = split_sign(a);
    let (b_neg, b_digits) = split_sign(b);
    match (a_neg, b_neg) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => cmp_digits(a_digits, b_digits),
        (true, true) => cmp_digits(b_digits, a_digits),
    }
}

fn split_sign(value: &str) -> (bool, &str) {
    match value.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, value),
    }
}

fn cmp_digits(a: &str, b: &str) -> Ordering {
    let a = a.trim_start_matches('0');
    let b = b.trim_start_matches('0');
    if a.len() != b.len() {
        return a.len().cmp(&b.len());
    }
    a.cmp(b)
}

/// Add one to a non-negative canonical digit string.
pub fn int_add_one(digits: &str) -> String {
    let mut bytes: Vec<u8> = digits.bytes().collect();
    let mut index = bytes.len();
    loop {
        if index == 0 {
            bytes.insert(0, b'1');
            break;
        }
        index -= 1;
        if bytes[index] == b'9' {
            bytes[index] = b'0';
        } else {
            bytes[index] += 1;
            break;
        }
    }
    String::from_utf8(bytes).expect("digits stay digits")
}

/// Add two canonical digit strings exactly (signs
/// handled), for token and count sums.
pub fn int_add(a: &str, b: &str) -> String {
    let (a_neg, a_digits) = split_sign(a);
    let (b_neg, b_digits) = split_sign(b);
    if a_neg == b_neg {
        let sum = add_digits(a_digits, b_digits);
        if a_neg && sum != "0" {
            format!("-{}", sum)
        } else {
            sum
        }
    } else {
        match cmp_digits(a_digits, b_digits) {
            Ordering::Equal => "0".to_string(),
            Ordering::Greater => {
                let diff = sub_digits(a_digits, b_digits);
                if a_neg {
                    format!("-{}", diff)
                } else {
                    diff
                }
            }
            Ordering::Less => {
                let diff = sub_digits(b_digits, a_digits);
                if b_neg {
                    format!("-{}", diff)
                } else {
                    diff
                }
            }
        }
    }
}

fn add_digits(a: &str, b: &str) -> String {
    let mut result: Vec<u8> = Vec::new();
    let mut carry = 0u8;
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut i = a.len();
    let mut j = b.len();
    while i > 0 || j > 0 || carry > 0 {
        let mut sum = carry;
        if i > 0 {
            i -= 1;
            sum += a[i] - b'0';
        }
        if j > 0 {
            j -= 1;
            sum += b[j] - b'0';
        }
        result.push(b'0' + (sum % 10));
        carry = sum / 10;
    }
    result.reverse();
    let text = String::from_utf8(result).expect("digits stay digits");
    let trimmed = text.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

fn sub_digits(a: &str, b: &str) -> String {
    // Requires a >= b as digit strings.
    let mut result: Vec<u8> = Vec::new();
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut borrow = 0i16;
    let mut i = a.len();
    let mut j = b.len();
    while i > 0 {
        i -= 1;
        let mut digit = a[i] as i16 - b'0' as i16 - borrow;
        if j > 0 {
            j -= 1;
            digit -= b[j] as i16 - b'0' as i16;
        }
        if digit < 0 {
            digit += 10;
            borrow = 1;
        } else {
            borrow = 0;
        }
        result.push(b'0' + digit as u8);
    }
    result.reverse();
    let text = String::from_utf8(result).expect("digits stay digits");
    let trimmed = text.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}
