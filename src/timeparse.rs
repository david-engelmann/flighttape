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

    /// The digit run at the cursor, consumed.
    fn digit_run(&mut self) -> &'a [u8] {
        let start = self.pos;
        while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
            self.pos += 1;
        }
        &self.bytes[start..self.pos]
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
    // One separator character, anything but NUL;
    // the time must follow it.
    let separator = text[cursor.pos..].chars().next()?;
    if separator == '\0' {
        return None;
    }
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
    // fixes the mode for every later component.
    if cursor.peek() == Some(b':') {
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
    if mode == Mode::Extended && cursor.peek() == Some(b':') {
        cursor.pos += 1;
        second = cursor.digits(2)?;
        if second > 59 {
            return None;
        }
    } else if mode == Mode::Basic && cursor.two_digits_here() {
        second = cursor.digits(2)?;
        if second > 59 {
            return None;
        }
    }
    // Leftover digits after the last component: one
    // stray digit is read and ignored, as the
    // interpreter reads it; a longer run is the
    // fraction in the basic form and a refusal in
    // the extended.
    let mut fraction = 0i64;
    let mut remainder_seen = false;
    if matches!(cursor.peek(), Some(b) if b.is_ascii_digit()) {
        remainder_seen = true;
        let run = cursor.digit_run();
        if run.len() > 1 {
            if mode == Mode::Basic {
                fraction = fraction_micros(run);
            } else {
                return None;
            }
        }
    }
    // A fraction behind its point: the digits may be
    // empty when an offset follows, never at the end.
    if !remainder_seen && matches!(cursor.peek(), Some(b'.') | Some(b',')) {
        cursor.pos += 1;
        let run = cursor.digit_run();
        if run.is_empty() && cursor.at_end() {
            return None;
        }
        fraction = fraction_micros(run);
    }
    let offset = offset(cursor)?;
    if !cursor.at_end() {
        return None;
    }
    Some((hour, minute, second, fraction, offset))
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
    if mode == Mode::Extended && cursor.peek() == Some(b':') {
        cursor.pos += 1;
        seconds = cursor.digits(2)?;
    } else if mode == Mode::Basic && cursor.two_digits_here() {
        seconds = cursor.digits(2)?;
    }
    // Fields past two digits are trailing content,
    // which the caller refuses.
    let mut fraction = 0i64;
    if matches!(cursor.peek(), Some(b'.') | Some(b',')) {
        cursor.pos += 1;
        let run = cursor.digit_run();
        if run.is_empty() {
            return None;
        }
        fraction = fraction_micros(run);
    }
    let total_seconds = hours * 3600 + minutes * 60 + seconds;
    if total_seconds >= 86_400 {
        return None;
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
