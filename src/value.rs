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

/// Python's truthiness for a JSON value: None, False,
/// numeric zero, and empty containers and strings are
/// false; NaN is true, as Python reads it.
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Int(digits) => digits != "0",
        Value::Float(number) => *number != 0.0,
        Value::Str(text) => !text.units().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(pairs) => !pairs.is_empty(),
    }
}

/// Python's str() of a JSON value, as text: a string
/// is itself, True and False and None are their names,
/// numbers render as their repr, and containers
/// render as their repr.
pub fn py_str(value: &Value) -> Text {
    match value {
        Value::Str(text) => text.clone(),
        Value::Null => Text::from_str("None"),
        Value::Bool(true) => Text::from_str("True"),
        Value::Bool(false) => Text::from_str("False"),
        Value::Int(digits) => Text::from_str(digits),
        Value::Float(number) => Text::from_str(&crate::pyrepr::repr_float(*number)),
        Value::Array(_) | Value::Object(_) => {
            Text::from_str(&crate::pyrepr::repr_value(value))
        }
    }
}

/// Compare two texts as Python compares strings: by
/// code point, where a surrogate pair is one code
/// point and an unpaired unit is its own value. The
/// canonical form's key order is the unit order on
/// Text itself; this is the order sorted() gives.
pub fn scalar_cmp(a: &Text, b: &Text) -> Ordering {
    let scalars = |text: &Text| -> Vec<u32> {
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
                    0x10000
                        + (((unit as u32 - 0xD800) << 10)
                            | (units[index + 1] as u32 - 0xDC00)),
                );
                index += 2;
            } else {
                out.push(unit as u32);
                index += 1;
            }
        }
        out
    };
    scalars(a).cmp(&scalars(b))
}

/// Build an object value from (key, value) pairs in
/// the order given, through a helper so the command
/// modules read like the documents they build.
pub fn object(pairs: Vec<(&str, Value)>) -> Value {
    Value::Object(
        pairs
            .into_iter()
            .map(|(key, value)| (Text::from_str(key), value))
            .collect(),
    )
}

/// Set a key on an object value, keeping the key's
/// first position when it already stands, as a Python
/// dict assignment does.
pub fn set_key(object: &mut Value, key: &str, value: Value) {
    if let Value::Object(pairs) = object {
        match pairs.iter_mut().find(|(name, _)| name == &key) {
            Some((_, existing)) => *existing = value,
            None => pairs.push((Text::from_str(key), value)),
        }
    }
}

/// Python's subtraction for two numbers: exact for
/// two integers, a float otherwise, and None where
/// Python raises OverflowError (a huge integer mixed
/// with a float) for the caller to name.
pub fn num_sub(left: &Value, right: &Value) -> Option<Value> {
    // Computes right - left in the callers' terms; see
    // num_add for the shared conversion.
    num_add_signed(left, right, true)
}

/// Python's addition for two numbers, same terms.
pub fn num_add(left: &Value, right: &Value) -> Option<Value> {
    num_add_signed(left, right, false)
}

fn num_add_signed(left: &Value, right: &Value, subtract: bool) -> Option<Value> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => {
            if subtract {
                let negated = if b == "0" {
                    "0".to_string()
                } else if let Some(stripped) = b.strip_prefix('-') {
                    stripped.to_string()
                } else {
                    format!("-{}", b)
                };
                Some(Value::Int(int_add(a, &negated)))
            } else {
                Some(Value::Int(int_add(a, b)))
            }
        }
        _ => {
            let a = to_float_checked(left)?;
            let b = to_float_checked(right)?;
            Some(Value::Float(if subtract { b - a } else { a + b }))
        }
    }
}

fn to_float_checked(value: &Value) -> Option<f64> {
    match value {
        Value::Int(digits) => {
            let parsed: f64 = digits.parse().ok()?;
            if parsed.is_infinite() {
                None
            } else {
                Some(parsed)
            }
        }
        Value::Float(number) => Some(*number),
        _ => None,
    }
}
