//! Timestamp parsing at the validator's fidelity:
//! Python's datetime.fromisoformat after the Z
//! replacement the validator performs. The grammar
//! was pinned against the interpreter itself over a
//! systematic corpus (tests/fixtures/timestamp_corpus.txt):
//! calendar and ISO week dates in extended and basic
//! forms, one arbitrary separator character, reduced
//! times, fractions whose digits land in the
//! microsecond field whatever component carries
//! them, and offsets whose fields sum to under a
//! day. An instant is i128 microseconds since the
//! Unix epoch, so ordering and durations are exact.

/// Parse an ISO 8601 timestamp the way the
/// validator's parse_timestamp does. Returns
/// microseconds since the epoch (UTC), or None.
pub fn parse_timestamp(value: &str) -> Option<i128> {
    let replaced = value.replace('Z', "+00:00");
    parse_iso(&replaced)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_at(&self, ahead: usize) -> Option<u8> {
        self.bytes.get(self.pos + ahead).copied()
    }

    fn at_end(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    /// Exactly n ASCII digits, as a number.
    fn digits(&mut self, count: usize) -> Option<i64> {
        let mut value: i64 = 0;
        for _ in 0..count {
            let byte = self.peek()?;
            if !byte.is_ascii_digit() {
                return None;
            }
            value = value * 10 + (byte - b'0') as i64;
            self.pos += 1;
        }
        Some(value)
    }

    /// Two digits stand at the cursor.
    fn two_digits_here(&self) -> bool {
        matches!(self.peek(), Some(b) if b.is_ascii_digit())
            && matches!(self.peek_at(1), Some(b) if b.is_ascii_digit())
    }
}

/// Fraction digits as microseconds: the first six
/// digits, right padded with zeros, whatever unit
/// the fraction was written against (fromisoformat
/// places the digits in the microsecond field).
fn fraction_micros(digits: &[u8]) -> i64 {
    let mut value: i64 = 0;
    let mut places = 0;
    for byte in digits.iter().take(6) {
        value = value * 10 + (byte - b'0') as i64;
        places += 1;
    }
    while places < 6 {
        value *= 10;
        places += 1;
    }
    value
}

fn parse_iso(text: &str) -> Option<i128> {
    let mut cursor = Cursor {
        bytes: text.as_bytes(),
        pos: 0,
    };
    let days = date(&mut cursor)?;
    let mut micros = days.checked_mul(86_400_000_000)?;
    if cursor.at_end() {
        return Some(micros);
    }
    // One separator character, any single character
    // at all; the time must follow it. NUL is a
    // separator like any other (FT-68): the Round 12
    // clause that barred it measured only the
    // positions where a full time did not follow.
    let separator = text[cursor.pos..].chars().next()?;
    cursor.pos += separator.len_utf8();
    let (hour, minute, second, fraction, offset) = time(&mut cursor)?;
    micros += (hour * 3600 + minute * 60 + second) as i128 * 1_000_000;
    micros += fraction as i128;
    micros -= offset;
    Some(micros)
}

/// The date forms: calendar extended YYYY-MM-DD and
/// basic YYYYMMDD, week extended YYYY-Www[-D] and
/// basic YYYYWww[D]. Returns days since the epoch.
fn date(cursor: &mut Cursor) -> Option<i128> {
    let year = cursor.digits(4)?;
    if year < 1 {
        return None;
    }
    if cursor.peek() == Some(b'W') {
        cursor.pos += 1;
        return week_date(cursor, year, false);
    }
    if cursor.peek() == Some(b'-') && cursor.peek_at(1) == Some(b'W') {
        cursor.pos += 2;
        return week_date(cursor, year, true);
    }
    let (month, day);
    if cursor.peek() == Some(b'-') {
        cursor.pos += 1;
        month = cursor.digits(2)?;
        if cursor.peek() != Some(b'-') {
            return None;
        }
        cursor.pos += 1;
        day = cursor.digits(2)?;
    } else if matches!(cursor.peek(), Some(b) if b.is_ascii_digit()) {
        month = cursor.digits(2)?;
        day = cursor.digits(2)?;
    } else {
        return None;
    }
    if !(1..=12).contains(&month) {
        return None;
    }
    let days_in_month = [
        31,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ][(month - 1) as usize];
    if day < 1 || day > days_in_month {
        return None;
    }
    Some(days_from_civil(year, month, day) as i128)
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// An ISO week date once the year and the W are
/// consumed. The day defaults to Monday; in the
/// extended form a dash before the day only counts
/// as the day's when a lone day digit follows it,
/// else the dash is the date and time separator.
fn week_date(cursor: &mut Cursor, year: i64, extended: bool) -> Option<i128> {
    let week = cursor.digits(2)?;
    if week < 1 || week > iso_weeks_in_year(year) {
        return None;
    }
    let mut day: i64 = 1;
    if extended {
        let day_ahead = cursor.peek() == Some(b'-')
            && matches!(cursor.peek_at(1), Some(b) if (b'1'..=b'7').contains(&b))
            && !matches!(cursor.peek_at(2), Some(b) if b.is_ascii_digit());
        if day_ahead {
            cursor.pos += 1;
            day = cursor.digits(1)?;
        }
    } else if matches!(cursor.peek(), Some(b) if b.is_ascii_digit()) {
        day = cursor.digits(1)?;
        if !(1..=7).contains(&day) {
            return None;
        }
    }
    let jan4 = days_from_civil(year, 1, 4);
    let jan4_weekday = (jan4 + 3).rem_euclid(7);
    let week_one_monday = jan4 - jan4_weekday;
    Some((week_one_monday + (week - 1) * 7 + (day - 1)) as i128)
}

fn iso_weeks_in_year(year: i64) -> i64 {
    // 53 when January 1 is a Thursday, or a leap
    // year's January 1 is a Wednesday.
    let jan1 = days_from_civil(year, 1, 1);
    let weekday = (jan1 + 3).rem_euclid(7);
    if weekday == 3 || (weekday == 2 && is_leap(year)) {
        53
    } else {
        52
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    None,
    Extended,
    Basic,
}

/// The time forms, with the offset. Returns the
/// hour, minute, second, the fraction's microseconds
/// and the signed offset in microseconds.
fn time(cursor: &mut Cursor) -> Option<(i64, i64, i64, i64, i128)> {
    let hour = cursor.digits(2)?;
    if hour > 23 {
        return None;
    }
    let mut minute = 0i64;
    let mut second = 0i64;
    let mut mode = Mode::None;
    // The first separator, a colon or bare digits,
    // fixes the mode for every later component. A
    // colon standing immediately before the offset's
    // sign introduces a component of zero instead
    // (FT-60a), for the minute and the second alike.
    if colon_before_sign(cursor) {
        cursor.pos += 1;
        mode = Mode::Extended;
    } else if cursor.peek() == Some(b':') {
        cursor.pos += 1;
        mode = Mode::Extended;
        minute = cursor.digits(2)?;
        if minute > 59 {
            return None;
        }
    } else if cursor.two_digits_here() {
        mode = Mode::Basic;
        minute = cursor.digits(2)?;
        if minute > 59 {
            return None;
        }
    }
    let mut seconds_read = false;
    if mode == Mode::Extended && colon_before_sign(cursor) {
        cursor.pos += 1;
    } else if mode == Mode::Extended && cursor.peek() == Some(b':') {
        cursor.pos += 1;
        second = cursor.digits(2)?;
        if second > 59 {
            return None;
        }
        seconds_read = true;
    } else if mode == Mode::Basic && cursor.two_digits_here() {
        second = cursor.digits(2)?;
        if second > 59 {
            return None;
        }
        seconds_read = true;
    }
    // The tail after the last component, up to the
    // offset boundary: the first sign byte of what
    // remains (FT-65). The tail carries the stray
    // digit (FT-60), the skipped separator byte
    // (FT-60a), and the fraction behind its point or
    // behind the colon that stands after the seconds
    // (FT-63), read under the window law (FT-64).
    let rest = &cursor.bytes[cursor.pos..];
    let boundary = rest
        .iter()
        .position(|byte| *byte == b'+' || *byte == b'-')
        .unwrap_or(rest.len());
    let fraction = tail_fraction(&rest[..boundary], mode, seconds_read, boundary < rest.len())?;
    cursor.pos += boundary;
    let offset = offset(cursor)?;
    if !cursor.at_end() {
        return None;
    }
    Some((hour, minute, second, fraction, offset))
}

/// The tail of a time, from just after its last
/// component to the offset boundary. `has_offset`
/// says a sign follows the tail.
fn tail_fraction(tail: &[u8], mode: Mode, seconds_read: bool, has_offset: bool) -> Option<i64> {
    if tail.is_empty() {
        return Some(0);
    }
    if tail.len() == 1 {
        // The separator candidate step (FT-60, FT-60a):
        // one character after the last component,
        // consumed and ignored when the offset follows
        // it. It wears every dress alike, a stray
        // digit, an empty fraction's point, a colon,
        // any other ASCII byte; at the end of the
        // string it refuses in all of them but one,
        // the lone NUL, which the interpreter reads
        // as the string's end (FT-68). A multi-byte
        // character is never one byte and is not
        // skipped.
        return if (has_offset && tail[0].is_ascii()) || tail[0] == 0 {
            Some(0)
        } else {
            None
        };
    }
    let first = tail[0];
    // A fraction behind its point, at any component.
    if first == b'.' || first == b',' {
        return fraction_run(&tail[1..], true, has_offset);
    }
    if first == b':' {
        // After the seconds the colon is the fraction
        // separator (FT-63); in a basic time it lands
        // inside the fraction window instead.
        if mode == Mode::Extended && seconds_read {
            return fraction_run(&tail[1..], true, has_offset);
        }
        if mode == Mode::Basic {
            return fraction_run(tail, false, has_offset);
        }
        return None;
    }
    if first.is_ascii_digit() {
        let run_len = tail.iter().take_while(|byte| byte.is_ascii_digit()).count();
        // One stray digit followed by more of the tail
        // refuses; the lone stray digit was the
        // candidate step's business above (FT-60).
        if run_len == 1 {
            return None;
        }
        // A longer digit run is the fraction in the
        // basic form, once the seconds stand read, and
        // a refusal in the extended.
        if mode == Mode::Basic && seconds_read {
            return fraction_run(tail, false, has_offset);
        }
        return None;
    }
    // Any other character here stands two or more
    // deep in the tail, past the candidate step, and
    // refuses.
    None
}

/// The fraction's characters after its separator
/// (or, in a basic time, after the seconds), under
/// the window law (FT-64, FT-65): of the characters
/// remaining, exactly min(R, 6) are read and all
/// must be digits; digits past the window are
/// skipped; anything left after the skipped digits
/// is unexamined leftover, which stands when an
/// offset follows it, or when the leftover's first
/// byte is NUL, the interpreter's end of string
/// (FT-68). A NUL later in the leftover, behind
/// other junk, does not stand.
fn fraction_run(rest: &[u8], separated: bool, has_offset: bool) -> Option<i64> {
    if rest.is_empty() {
        // An empty fraction stands only before an
        // offset sign, never at the end of the string.
        return if separated && has_offset {
            Some(0)
        } else {
            None
        };
    }
    let window = rest.len().min(6);
    if !rest[..window].iter().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let fraction = fraction_micros(&rest[..window]);
    if rest.len() > 6 {
        let mut end = 6;
        while end < rest.len() && rest[end].is_ascii_digit() {
            end += 1;
        }
        if end < rest.len() && !has_offset && rest[end] != 0 {
            return None;
        }
    }
    Some(fraction)
}

/// A component colon standing immediately before
/// the offset's sign: the component it introduces
/// reads as zero and the offset follows (FT-60a).
fn colon_before_sign(cursor: &Cursor) -> bool {
    cursor.peek() == Some(b':') && matches!(cursor.peek_at(1), Some(b'+') | Some(b'-'))
}

/// The offset forms: a signed HH with optional
/// minutes and seconds in the mode the first
/// separator fixes, strict two digit fields, an
/// optional fraction that needs at least one digit,
/// and a total strictly under a day. Returns signed
/// microseconds.
fn offset(cursor: &mut Cursor) -> Option<i128> {
    if cursor.at_end() {
        return Some(0);
    }
    let sign: i128 = match cursor.peek() {
        Some(b'+') => 1,
        Some(b'-') => -1,
        _ => return None,
    };
    cursor.pos += 1;
    let hours = cursor.digits(2)?;
    let mut minutes = 0i64;
    let mut seconds = 0i64;
    let mut mode = Mode::None;
    if cursor.peek() == Some(b':') {
        cursor.pos += 1;
        mode = Mode::Extended;
        minutes = cursor.digits(2)?;
    } else if cursor.two_digits_here() {
        mode = Mode::Basic;
        minutes = cursor.digits(2)?;
    }
    let mut seconds_read = false;
    if mode == Mode::Extended && cursor.peek() == Some(b':') {
        cursor.pos += 1;
        seconds = cursor.digits(2)?;
        seconds_read = true;
    } else if mode == Mode::Basic && cursor.two_digits_here() {
        seconds = cursor.digits(2)?;
        seconds_read = true;
    }
    // The offset's fraction, behind its point or
    // behind the colon that stands after the offset
    // seconds (FT-63). The offset is consumed
    // exactly: the fraction needs at least one
    // digit, and nothing may stand after its digits
    // (FT-65), save the FT-68 suffix below.
    let mut fraction = 0i64;
    let separated = matches!(cursor.peek(), Some(b'.') | Some(b','))
        || (cursor.peek() == Some(b':') && mode == Mode::Extended && seconds_read);
    if separated {
        cursor.pos += 1;
        let rest = &cursor.bytes[cursor.pos..];
        let run = rest.iter().take_while(|byte| byte.is_ascii_digit()).count();
        if run == 0 {
            return None;
        }
        fraction = fraction_micros(&rest[..run]);
        cursor.pos += run;
        let suffix = &rest[run..];
        if !suffix.is_empty() {
            // Past the digit run the offset is consumed
            // exactly, with one exception (FT-68): when
            // the run filled the window, a suffix whose
            // first byte is NUL is the interpreter's end
            // of string, and the rest is ignored whole.
            if run >= 6 && suffix[0] == 0 {
                cursor.pos = cursor.bytes.len();
            } else {
                return None;
            }
        }
    }
    let total_seconds = hours * 3600 + minutes * 60 + seconds;
    if total_seconds >= 86_400 {
        return None;
    }
    if !separated && cursor.bytes[cursor.pos..] == [0] {
        // A lone NUL after the offset's last component
        // is the string's end (FT-68); anything else
        // standing there is the caller's to refuse.
        cursor.pos += 1;
    }
    if total_seconds == 0 {
        // A zero offset is the UTC fast path in the
        // interpreter, and its fraction is dropped.
        fraction = 0;
    }
    Some(sign * (total_seconds as i128 * 1_000_000 + fraction as i128))
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
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146097 + day_of_era - 719468
}
