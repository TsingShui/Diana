//! Canonical `at` locators and display escaping.
//!
//! The locator is the original identity: physical entry, logical DEX, class_def
//! or member index plus the verifiable descriptor/name. Rendering never depends on
//! result order.

/// Decodes UTF-16 units for display, escaping lone surrogates instead of
/// replacing them.
pub fn escape_units(units: &[u16]) -> String {
    let mut text = String::with_capacity(units.len());
    let mut index = 0;
    while index < units.len() {
        let unit = units[index];
        if (0xd800..0xdc00).contains(&unit) {
            if let Some(&next) = units.get(index + 1)
                && (0xdc00..0xe000).contains(&next)
                && let Some(character) = char::from_u32(
                    0x1_0000 + ((u32::from(unit) - 0xd800) << 10) + (u32::from(next) - 0xdc00),
                )
            {
                push_escaped(&mut text, character);
                index += 2;
                continue;
            }
            text.push_str(&format!("\\u{unit:04x}"));
            index += 1;
            continue;
        }
        if (0xdc00..0xe000).contains(&unit) {
            text.push_str(&format!("\\u{unit:04x}"));
            index += 1;
            continue;
        }
        if let Some(character) = char::from_u32(u32::from(unit)) {
            push_escaped(&mut text, character);
        }
        index += 1;
    }
    text
}

fn push_escaped(text: &mut String, character: char) {
    match character {
        '\\' => text.push_str("\\\\"),
        '\t' => text.push_str("\\t"),
        '\r' => text.push_str("\\r"),
        '\n' => text.push_str("\\n"),
        '\0' => text.push_str("\\0"),
        other => text.push(other),
    }
}

fn prefix(entry: Option<usize>, logical: u32) -> String {
    // A bare DEX is the single physical entry `e0`.
    format!("e{}:l{logical}", entry.unwrap_or(0))
}

pub fn class(entry: Option<usize>, logical: u32, class_def: u32, descriptor: &[u16]) -> String {
    format!(
        "{}:c{class_def}::{}",
        prefix(entry, logical),
        escape_units(descriptor)
    )
}

pub fn method(
    entry: Option<usize>,
    logical: u32,
    class_def: u32,
    method: u32,
    descriptor: &[u16],
    name: &[u16],
    prototype: &[u16],
) -> String {
    format!(
        "{}:c{class_def}:m{method}::{}->{}{}",
        prefix(entry, logical),
        escape_units(descriptor),
        escape_units(name),
        escape_units(prototype)
    )
}

pub fn method_index(entry: Option<usize>, logical: u32, class_def: u32, method: u32) -> String {
    format!("{}:c{class_def}:m{method}", prefix(entry, logical))
}

pub fn field(
    entry: Option<usize>,
    logical: u32,
    class_def: u32,
    field: u32,
    descriptor: &[u16],
    name: &[u16],
    field_type: &[u16],
) -> String {
    format!(
        "{}:c{class_def}:f{field}::{}->{}:{}",
        prefix(entry, logical),
        escape_units(descriptor),
        escape_units(name),
        escape_units(field_type)
    )
}

pub fn string(entry: Option<usize>, logical: u32, string: u32) -> String {
    format!("{}:s{string}", prefix(entry, logical))
}

/// Which escaping a published field needs.
///
/// Both modes write the display form and then escape it as a column: a text column
/// is escaped once more so the field never spans two lines, and a JSON column is
/// escaped as a JSON string. Text and JSON therefore differ only in how a `"` is
/// written, which is what the published bytes have always done.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Mode {
    Text,
    Json,
}

/// Appends the `e<entry>:l<logical>` locator prefix.
pub fn write_prefix(out: &mut Vec<u8>, entry: Option<usize>, logical: u32) {
    out.push(b'e');
    push_decimal(out, entry.unwrap_or(0) as u64);
    out.extend_from_slice(b":l");
    push_decimal(out, u64::from(logical));
}

/// Appends the decimal form of `value` without allocating.
pub fn push_decimal(out: &mut Vec<u8>, value: u64) {
    let mut digits = [0u8; 20];
    let mut at = digits.len();
    let mut rest = value;
    loop {
        at -= 1;
        digits[at] = b'0' + (rest % 10) as u8;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    out.extend_from_slice(&digits[at..]);
}

/// Appends one DEX string value with display escapes, directly from its bytes.
///
/// A value that is already UTF-8 is emitted from the borrowed payload; only a
/// surrogate-bearing value (MUTF-8/CESU-8, which is not UTF-8) materializes its
/// units, so a whole string pool costs no per-value allocation.
pub fn write_value(
    out: &mut Vec<u8>,
    mode: Mode,
    value: diana_decomp::input::DexString<'_>,
) -> Result<(), diana_decomp::input::InputError> {
    // Establish ASCII and absence of escapes in one pass. Most descriptors can
    // then be copied without a second byte scan or a UTF-8 conversion.
    if plain_ascii(value.bytes(), mode) {
        out.extend_from_slice(value.bytes());
        return Ok(());
    }
    match value.utf8() {
        Some(text) => {
            write_text(out, mode, text);
            Ok(())
        }
        None => {
            write_units(out, mode, &value.units()?);
            Ok(())
        }
    }
}

/// Appends text with display escapes.
///
/// Runs that need nothing are copied whole and only the bytes that change go
/// through a match: a method row is ~100 characters of locator, signature and name,
/// and a per-character helper for every one of them was the largest single cost of
/// `methods`. Every escape-relevant byte is ASCII, so scanning bytes cannot split a
/// multi-byte character.
pub fn write_text(out: &mut Vec<u8>, mode: Mode, text: &str) {
    write_bytes(out, mode, text.as_bytes());
}

/// Appends bytes with display escapes.
///
/// Every escape-relevant byte is ASCII, so escaping arbitrary input cannot split a
/// multi-byte character and one byte scan serves text, JSON and raw pool payloads.
pub fn write_bytes(out: &mut Vec<u8>, mode: Mode, bytes: &[u8]) {
    if plain_ascii(bytes, mode) {
        out.extend_from_slice(bytes);
        return;
    }
    let mut run = 0usize;
    for (index, &byte) in bytes.iter().enumerate() {
        if !needs_escape(mode, byte) {
            continue;
        }
        out.extend_from_slice(&bytes[run..index]);
        push_escape(out, mode, byte);
        run = index + 1;
    }
    out.extend_from_slice(&bytes[run..]);
}

/// Checks eight bytes at a time using safe, unaligned-by-copy loads. The zero-byte
/// detector can conservatively flag a neighbouring byte after a borrow; that only
/// selects the ordinary writer. It never overlooks a control, non-ASCII byte,
/// backslash, or (in JSON) quote. Wrapping subtraction is intentional bit arithmetic.
fn plain_ascii(bytes: &[u8], mode: Mode) -> bool {
    const HIGH: u64 = 0x8080_8080_8080_8080;
    const ONES: u64 = 0x0101_0101_0101_0101;
    let (words, tail) = bytes.as_chunks::<8>();
    for word in words {
        let word = u64::from_le_bytes(*word);
        let slashes = word ^ (u64::from(b'\\') * ONES);
        let mut special = word
            | (word.wrapping_sub(0x2020_2020_2020_2020) & !word)
            | (slashes.wrapping_sub(ONES) & !slashes);
        if mode == Mode::Json {
            let quotes = word ^ (u64::from(b'"') * ONES);
            special |= quotes.wrapping_sub(ONES) & !quotes;
        }
        if special & HIGH != 0 {
            return false;
        }
    }
    tail.iter().all(|&byte| {
        (0x20..0x80).contains(&byte) && byte != b'\\' && (mode != Mode::Json || byte != b'"')
    })
}

fn needs_escape(mode: Mode, byte: u8) -> bool {
    match mode {
        Mode::Text => matches!(byte, b'\\' | b'\t' | b'\r' | b'\n'),
        Mode::Json => matches!(byte, b'\\' | b'\t' | b'\r' | b'\n' | b'"') || byte < 0x20,
    }
}

/// Appends already decoded units with display escapes.
pub fn write_units(out: &mut Vec<u8>, mode: Mode, units: &[u16]) {
    let mut index = 0;
    while index < units.len() {
        let unit = units[index];
        if (0xd800..0xdc00).contains(&unit) {
            if let Some(&next) = units.get(index + 1)
                && (0xdc00..0xe000).contains(&next)
                && let Some(character) = char::from_u32(
                    0x1_0000 + ((u32::from(unit) - 0xd800) << 10) + (u32::from(next) - 0xdc00),
                )
            {
                push_char(out, mode, character);
                index += 2;
                continue;
            }
            push_surrogate_escape(out, unit);
            index += 1;
            continue;
        }
        if (0xdc00..0xe000).contains(&unit) {
            push_surrogate_escape(out, unit);
            index += 1;
            continue;
        }
        if let Some(character) = char::from_u32(u32::from(unit)) {
            push_char(out, mode, character);
        }
        index += 1;
    }
}

fn push_char(out: &mut Vec<u8>, mode: Mode, character: char) {
    if character.is_ascii() {
        let byte = character as u8;
        if !needs_escape(mode, byte) && byte >= 0x20 {
            out.push(byte);
            return;
        }
        match mode {
            Mode::Text if byte < 0x20 && !matches!(byte, b'\t' | b'\r' | b'\n' | 0) => {
                // Text keeps control characters other than the four display escapes.
                out.push(byte);
                return;
            }
            _ => push_escape(out, mode, byte),
        }
        return;
    }
    let mut buffer = [0u8; 4];
    out.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
}

/// One byte's escape: the display form, then the column escaping of both modes.
fn push_escape(out: &mut Vec<u8>, mode: Mode, byte: u8) {
    match byte {
        b'\\' => out.extend_from_slice(b"\\\\\\\\"),
        b'\t' => out.extend_from_slice(b"\\\\t"),
        b'\r' => out.extend_from_slice(b"\\\\r"),
        b'\n' => out.extend_from_slice(b"\\\\n"),
        0 => out.extend_from_slice(b"\\\\0"),
        b'"' if mode == Mode::Json => out.extend_from_slice(b"\\\""),
        other => match mode {
            Mode::Text => out.push(other),
            Mode::Json => push_control_escape(out, other),
        },
    }
}

/// Writes `\u00xx` for a raw control byte.
fn push_control_escape(out: &mut Vec<u8>, byte: u8) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out.extend_from_slice(b"\\u00");
    out.push(HEX[usize::from(byte >> 4)]);
    out.push(HEX[usize::from(byte & 0x0f)]);
}

fn push_surrogate_escape(out: &mut Vec<u8>, unit: u16) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    // The display form already reads `\uXXXX`; the column escaper doubles its
    // backslash in both modes.
    out.extend_from_slice(b"\\\\u");
    for shift in [12, 8, 4, 0] {
        out.push(HEX[usize::from((unit >> shift) & 0x0f)]);
    }
}
