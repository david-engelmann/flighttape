//! Timestamp parsing at the validator's fidelity:
//! Python's datetime.fromisoformat after the Z
//! replacement the validator performs, over the
//! grammar tape timestamps actually carry. An instant
//! is i128 microseconds since the Unix epoch, so
//! ordering and durations are exact.

/// Parse an ISO 8601 timestamp the way the
/// validator's parse_timestamp does. Returns
/// microseconds since the epoch (UTC), or None.
pub fn parse_timestamp(value: &str) -> Option<i128> {
    let replaced = value.replace('Z', "+00:00");
    parse_iso(&replaced)
}

fn parse_iso(text: &str) -> Option<i128> {
    let bytes = text.as_bytes();
    if bytes.len() < 10 {
        return None;
    }
    let digit = |index: usize| -> Option<i64> {
        let byte = *bytes.get(index)?;
        if byte.is_ascii_digit() {
            Some((byte - b'0') as i64)
        } else {
            None
        }
    };
    let two = |index: usize| -> Option<i64> {
        Some(digit(index)? * 10 + digit(index + 1)?)
    };
    let four = |index: usize| -> Option<i64> {
        Some(digit(index)? * 1000 + digit(index + 1)? * 100 + digit(index + 2)? * 10 + digit(index + 3)?)
    };
    // Date: YYYY-MM-DD (the compact YYYYMMDD form is
    // also ISO; fromisoformat accepts it).
    let (year, month, day, mut pos);
    if bytes.get(4) == Some(&b'-') {
        year = four(0)?;
        month = two(5)?;
        day = two(8)?;
        pos = 10;
    } else {
        year = four(0)?;
        month = two(4)?;
        day = two(6)?;
        pos = 8;
    }
    if !(1..=12).contains(&month) {
        return None;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days_in_month = [
        31,
        if leap { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ][(month - 1) as usize];
    if day < 1 || day > days_in_month {
        return None;
    }
    let mut hour = 0i64;
    let mut minute = 0i64;
    let mut second = 0i64;
    let mut micros = 0i64;
    let mut offset_seconds: Option<i64> = None;
    if pos < bytes.len() {
        // One separator character (T, space, or any
        // single character, as fromisoformat allows).
        pos += 1;
        hour = two(pos)?;
        if hour > 24 || (hour == 24 && bytes.get(pos + 3) != Some(&b':')) {
            // fromisoformat refuses hour 24 outside the
            // end-of-day midnight form; the tape grammar
            // never carries it, refuse it whole.
            if hour >= 24 {
                return None;
            }
        }
        pos += 2;
        if bytes.get(pos) != Some(&b':') {
            return None;
        }
        minute = two(pos + 1)?;
        if minute > 59 {
            return None;
        }
        pos += 3;
        if bytes.get(pos) == Some(&b':') {
            second = two(pos + 1)?;
            if second > 60 {
                return None;
            }
            pos += 3;
        }
        if bytes.get(pos) == Some(&b'.') || bytes.get(pos) == Some(&b',') {
            pos += 1;
            let start = pos;
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            if pos == start {
                return None;
            }
            let fraction = &text[start..pos];
            let mut digits = fraction.to_string();
            digits.truncate(6);
            while digits.len() < 6 {
                digits.push('0');
            }
            micros = digits.parse::<i64>().ok()?;
        }
        if pos < bytes.len() {
            // Offset: Z already replaced; +/-HH:MM,
            // +/-HHMM, or +/-HH.
            let sign = match bytes[pos] {
                b'+' => 1i64,
                b'-' => -1i64,
                _ => return None,
            };
            pos += 1;
            let offset_hour = two(pos)?;
            pos += 2;
            let mut offset_minute = 0i64;
            let mut offset_second = 0i64;
            if bytes.get(pos) == Some(&b':') {
                offset_minute = two(pos + 1)?;
                pos += 3;
                if bytes.get(pos) == Some(&b':') {
                    offset_second = two(pos + 1)?;
                    pos += 3;
                }
            } else if pos + 1 < bytes.len()
                && bytes[pos].is_ascii_digit()
                && bytes[pos + 1].is_ascii_digit()
            {
                offset_minute = two(pos)?;
                pos += 2;
            }
            if offset_hour > 23 || offset_minute > 59 || offset_second > 59 {
                return None;
            }
            offset_seconds =
                Some(sign * (offset_hour * 3600 + offset_minute * 60 + offset_second));
        }
    }
    if pos != bytes.len() {
        return None;
    }
    let days = days_from_civil(year, month, day);
    let local = days * 86_400 + hour * 3600 + minute * 60 + second;
    let utc = local - offset_seconds.unwrap_or(0);
    Some(utc as i128 * 1_000_000 + micros as i128)
}

/// Days since 1970-01-01 for a civil date (Hinnant's
/// algorithm), exact over the proleptic Gregorian
/// calendar Python uses.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let shifted = if month <= 2 { year - 1 } else { year };
    let era = if shifted >= 0 { shifted } else { shifted - 399 } / 400;
    let year_of_era = shifted - era * 400;
    let month_prime = (month + 9) % 12;
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era =
        year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146097 + day_of_era - 719468
}
