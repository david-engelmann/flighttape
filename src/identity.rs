//! Tape identity, the canonical form and the digest
//! (FT-26 to FT-28, FT-52, FT-54). The canonical form
//! is language portable on purpose: a verifier in any
//! language that follows FT-26 reaches the same bytes.

use sha2::{Digest, Sha256};

use crate::value::{Text, Value};

/// The tape cannot be canonicalized (FT-28).
#[derive(Clone, Debug, PartialEq)]
pub struct DigestError(pub String);

impl std::fmt::Display for DigestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DigestError {}

const RENDER_DEPTH_LIMIT: usize = 1000;
const INT_DIGIT_LIMIT: usize = 4300;

/// Render a finite float as ECMAScript renders it
/// (FT-26). The standard library's exponential form
/// gives the shortest digits that round trip; from
/// them, point is the count of digits before the
/// decimal point when the value is written out in
/// decimal, and the four cases at the end are the
/// ECMAScript number to string cases.
/// The shortest digits that round-trip, and the
/// decimal exponent of the first digit, for a finite
/// nonzero double. The exact expansion is computed by
/// integer arithmetic (FT-52): the value is m x 2^e,
/// and its decimal digits are m x 2^e or m x 5^-e by
/// repeated multiplication of a digit string. A cut
/// landing exactly on the half rounds to the even
/// digit, as the ECMAScript algorithm rounds it.
fn shortest_digits(value: f64) -> (String, i64) {
    let bits = value.abs().to_bits();
    let biased = ((bits >> 52) & 0x7FF) as i64;
    let fraction = bits & ((1u64 << 52) - 1);
    let (mantissa, exponent) = if biased == 0 {
        (fraction, -1074i64)
    } else {
        (fraction | (1u64 << 52), biased - 1075)
    };
    // The exact digits, least significant first, grown
    // by chunked multiplication (5^13 or 2^30 at a
    // step) so even the extreme exponents stay cheap.
    let mut digits: Vec<u8> = mantissa
        .to_string()
        .bytes()
        .map(|byte| byte - b'0')
        .rev()
        .collect();
    let point: i64 = if exponent >= 0 {
        let mut left = exponent;
        while left >= 30 {
            multiply_chunk(&mut digits, 1 << 30);
            left -= 30;
        }
        if left > 0 {
            multiply_chunk(&mut digits, 1 << left);
        }
        digits.len() as i64
    } else {
        let mut left = -exponent;
        while left >= 13 {
            multiply_chunk(&mut digits, 1_220_703_125);
            left -= 13;
        }
        if left > 0 {
            multiply_chunk(&mut digits, 5u32.pow(left as u32) as u64);
        }
        digits.len() as i64 + exponent
    };
    // Most significant first, as ASCII, for the cuts.
    let exact: Vec<u8> = digits.iter().rev().map(|digit| digit + b'0').collect();
    for width in 1..=17usize {
        let (candidate, candidate_point) = round_at(&exact, width, point);
        let probe = format!("{}e{}", candidate, candidate_point - candidate.len() as i64);
        if let Ok(parsed) = probe.parse::<f64>() {
            if parsed.to_bits() == value.abs().to_bits() {
                let trimmed = candidate.trim_end_matches('0');
                let trimmed = if trimmed.is_empty() { "0" } else { trimmed };
                return (trimmed.to_string(), candidate_point - 1);
            }
        }
    }
    unreachable!("every double has a 17-digit round trip")
}

/// Multiply a least-significant-first digit vector by
/// a chunk factor, in place.
fn multiply_chunk(digits: &mut Vec<u8>, factor: u64) {
    let mut carry: u64 = 0;
    for digit in digits.iter_mut() {
        let product = *digit as u64 * factor + carry;
        *digit = (product % 10) as u8;
        carry = product / 10;
    }
    while carry > 0 {
        digits.push((carry % 10) as u8);
        carry /= 10;
    }
}

/// Round the exact digits at the given width, half to
/// even on an exact tie. Returns the rounded digits
/// (exactly width of them, a carry may grow the point)
/// and the point of the rounded value.
fn round_at(exact: &[u8], width: usize, point: i64) -> (String, i64) {
    if exact.len() <= width {
        let mut digits = String::from_utf8(exact.to_vec()).expect("digits");
        while digits.len() < width {
            digits.push('0');
        }
        return (digits, point);
    }
    let kept: Vec<u8> = exact[..width].to_vec();
    let rest = &exact[width..];
    let round_up = if rest[0] > b'5' {
        true
    } else if rest[0] < b'5' {
        false
    } else if rest[1..].iter().any(|byte| *byte != b'0') {
        true
    } else {
        // Exactly on the half: round to even.
        (kept[width - 1] - b'0') % 2 == 1
    };
    if !round_up {
        return (String::from_utf8(kept).expect("digits"), point);
    }
    let mut digits = kept;
    let mut index = width;
    loop {
        if index == 0 {
            digits.insert(0, b'1');
            return (String::from_utf8(digits).expect("digits"), point + 1);
        }
        index -= 1;
        if digits[index] == b'9' {
            digits[index] = b'0';
        } else {
            digits[index] += 1;
            return (String::from_utf8(digits).expect("digits"), point);
        }
    }
}

pub fn render_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    let (digits, exp10) = shortest_digits(value);
    let point = exp10 + 1;
    let count = digits.len() as i64;
    if count <= point && point <= 21 {
        format!("{}{}{}", sign, digits, "0".repeat((point - count) as usize))
    } else if 0 < point && point <= 21 {
        format!(
            "{}{}.{}",
            sign,
            &digits[..point as usize],
            &digits[point as usize..]
        )
    } else if -6 < point && point <= 0 {
        format!("{}0.{}{}", sign, "0".repeat((-point) as usize), digits)
    } else {
        let head = if count == 1 {
            digits.to_string()
        } else {
            format!("{}.{}", &digits[..1], &digits[1..])
        };
        format!(
            "{}{}e{}{}",
            sign,
            head,
            if exp10 >= 0 { "+" } else { "-" },
            exp10.abs()
        )
    }
}

fn render(value: &Value, depth: usize, out: &mut Vec<u16>) -> Result<(), DigestError> {
    match value {
        Value::Null => out.extend("null".encode_utf16()),
        Value::Bool(true) => out.extend("true".encode_utf16()),
        Value::Bool(false) => out.extend("false".encode_utf16()),
        Value::Str(text) => text.render_json(out),
        Value::Int(digits) => {
            if digits.trim_start_matches('-').len() > INT_DIGIT_LIMIT {
                return Err(DigestError(
                    "the tape holds a value that cannot be rendered".to_string(),
                ));
            }
            out.extend(digits.encode_utf16());
        }
        Value::Float(number) => {
            if !number.is_finite() {
                return Err(DigestError(
                    "the tape holds a non-finite number".to_string(),
                ));
            }
            out.extend(render_number(*number).encode_utf16());
        }
        Value::Array(items) => {
            if depth >= RENDER_DEPTH_LIMIT {
                return Err(DigestError(
                    "the tape is nested beyond the interpreter limit".to_string(),
                ));
            }
            out.push(b'[' as u16);
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(b',' as u16);
                }
                render(item, depth + 1, out)?;
            }
            out.push(b']' as u16);
        }
        Value::Object(pairs) => {
            if depth >= RENDER_DEPTH_LIMIT {
                return Err(DigestError(
                    "the tape is nested beyond the interpreter limit".to_string(),
                ));
            }
            let mut ordered: Vec<&(Text, Value)> = pairs.iter().collect();
            ordered.sort_by(|a, b| a.0.cmp(&b.0));
            out.push(b'{' as u16);
            for (index, (key, item)) in ordered.iter().enumerate() {
                if index > 0 {
                    out.push(b',' as u16);
                }
                key.render_json(out);
                out.push(b':' as u16);
                render(item, depth + 1, out)?;
            }
            out.push(b'}' as u16);
        }
    }
    Ok(())
}

/// The tape's canonical form as UTF-8 bytes (FT-26).
pub fn canonical_bytes(tape: &Value) -> Result<Vec<u8>, DigestError> {
    let mut units: Vec<u16> = Vec::new();
    render(tape, 0, &mut units)?;
    match String::from_utf16(&units) {
        Ok(text) => Ok(text.into_bytes()),
        Err(_) => Err(DigestError(
            "the tape holds text that cannot be encoded".to_string(),
        )),
    }
}

/// The tape's digest, lowercase hex SHA-256 (FT-27).
pub fn tape_digest(tape: &Value) -> Result<String, DigestError> {
    let bytes = canonical_bytes(tape)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}
