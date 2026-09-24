//! Helpers for `Options::ENABLE_CMARK_GFM_COMPAT`: ports of the cmark-gfm
//! 0.29.0.gfm.13 functions that decide where it and CommonMark 0.31 part
//! ways. Each function names the C function it follows. See `UPLEFT.md` at
//! the repository root for the list of differences the option covers.

use std::ops::Range;

use unicase::UniCase;

use crate::linklabel::{scan_link_label_rest, LinkLabel};
use crate::parse::{Item, ItemBody};
use crate::scanners;
use crate::strings::CowStr;
use crate::tree::{Tree, TreeIndex};
use crate::Alignment;

/// `cmark_utf8proc_is_space`: tab, line feed, form feed, carriage return and
/// the `Zs` characters. (Unlike `char::is_whitespace`, not U+000B, U+0085,
/// U+2028 or U+2029.)
#[inline]
pub(crate) fn is_space(c: char) -> bool {
    matches!(
        c as u32,
        9 | 10 | 12 | 13 | 32 | 0xA0 | 0x1680 | 0x2000..=0x200A | 0x202F | 0x205F | 0x3000
    )
}

/// `cmark_utf8proc_is_punctuation`: ASCII punctuation and the Unicode `P*`
/// categories of cmark's table. Unlike CommonMark 0.31 it leaves out the
/// symbol categories (`S*`), so an emoji next to `*` is not punctuation.
#[inline]
pub(crate) fn is_punctuation(c: char) -> bool {
    let cp = c as u32;
    if cp < 128 {
        return (c as u8).is_ascii_punctuation();
    }
    if cp < 0xA1 || cp > 0x1BC9F {
        return false;
    }
    match PUNCTUATION.binary_search_by(|&(low, high)| {
        if high < cp {
            std::cmp::Ordering::Less
        } else if low > cp {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    }) {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// The non-ASCII ranges of `cmark_utf8proc_is_punctuation`, merged.
#[rustfmt::skip]
static PUNCTUATION: [(u32, u32); 146] = [
    (0x00A1, 0x00A1), (0x00A7, 0x00A7), (0x00AB, 0x00AB), (0x00B6, 0x00B7), (0x00BB, 0x00BB), (0x00BF, 0x00BF),
    (0x037E, 0x037E), (0x0387, 0x0387), (0x055A, 0x055F), (0x0589, 0x058A), (0x05BE, 0x05BE), (0x05C0, 0x05C0),
    (0x05C3, 0x05C3), (0x05C6, 0x05C6), (0x05F3, 0x05F4), (0x0609, 0x060A), (0x060C, 0x060D), (0x061B, 0x061B),
    (0x061E, 0x061F), (0x066A, 0x066D), (0x06D4, 0x06D4), (0x0700, 0x070D), (0x07F7, 0x07F9), (0x0830, 0x083E),
    (0x085E, 0x085E), (0x0964, 0x0965), (0x0970, 0x0970), (0x0AF0, 0x0AF0), (0x0DF4, 0x0DF4), (0x0E4F, 0x0E4F),
    (0x0E5A, 0x0E5B), (0x0F04, 0x0F12), (0x0F14, 0x0F14), (0x0F3A, 0x0F3D), (0x0F85, 0x0F85), (0x0FD0, 0x0FD4),
    (0x0FD9, 0x0FDA), (0x104A, 0x104F), (0x10FB, 0x10FB), (0x1360, 0x1368), (0x1400, 0x1400), (0x166D, 0x166E),
    (0x169B, 0x169C), (0x16EB, 0x16ED), (0x1735, 0x1736), (0x17D4, 0x17D6), (0x17D8, 0x17DA), (0x1800, 0x180A),
    (0x1944, 0x1945), (0x1A1E, 0x1A1F), (0x1AA0, 0x1AA6), (0x1AA8, 0x1AAD), (0x1B5A, 0x1B60), (0x1BFC, 0x1BFF),
    (0x1C3B, 0x1C3F), (0x1C7E, 0x1C7F), (0x1CC0, 0x1CC7), (0x1CD3, 0x1CD3), (0x2010, 0x2027), (0x2030, 0x2043),
    (0x2045, 0x2051), (0x2053, 0x205E), (0x207D, 0x207E), (0x208D, 0x208E), (0x2308, 0x230B), (0x2329, 0x232A),
    (0x2768, 0x2775), (0x27C5, 0x27C6), (0x27E6, 0x27EF), (0x2983, 0x2998), (0x29D8, 0x29DB), (0x29FC, 0x29FD),
    (0x2CF9, 0x2CFC), (0x2CFE, 0x2CFF), (0x2D70, 0x2D70), (0x2E00, 0x2E2E), (0x2E30, 0x2E42), (0x3001, 0x3003),
    (0x3008, 0x3011), (0x3014, 0x301F), (0x3030, 0x3030), (0x303D, 0x303D), (0x30A0, 0x30A0), (0x30FB, 0x30FB),
    (0xA4FE, 0xA4FF), (0xA60D, 0xA60F), (0xA673, 0xA673), (0xA67E, 0xA67E), (0xA6F2, 0xA6F7), (0xA874, 0xA877),
    (0xA8CE, 0xA8CF), (0xA8F8, 0xA8FA), (0xA92E, 0xA92F), (0xA95F, 0xA95F), (0xA9C1, 0xA9CD), (0xA9DE, 0xA9DF),
    (0xAA5C, 0xAA5F), (0xAADE, 0xAADF), (0xAAF0, 0xAAF1), (0xABEB, 0xABEB), (0xFD3E, 0xFD3F), (0xFE10, 0xFE19),
    (0xFE30, 0xFE52), (0xFE54, 0xFE61), (0xFE63, 0xFE63), (0xFE68, 0xFE68), (0xFE6A, 0xFE6B), (0xFF01, 0xFF03),
    (0xFF05, 0xFF0A), (0xFF0C, 0xFF0F), (0xFF1A, 0xFF1B), (0xFF1F, 0xFF20), (0xFF3B, 0xFF3D), (0xFF3F, 0xFF3F),
    (0xFF5B, 0xFF5B), (0xFF5D, 0xFF5D), (0xFF5F, 0xFF65), (0x10100, 0x10102), (0x1039F, 0x1039F), (0x103D0, 0x103D0),
    (0x1056F, 0x1056F), (0x10857, 0x10857), (0x1091F, 0x1091F), (0x1093F, 0x1093F), (0x10A50, 0x10A58), (0x10A7F, 0x10A7F),
    (0x10AF0, 0x10AF6), (0x10B39, 0x10B3F), (0x10B99, 0x10B9C), (0x11047, 0x1104D), (0x110BB, 0x110BC), (0x110BE, 0x110C1),
    (0x11140, 0x11143), (0x11174, 0x11175), (0x111C5, 0x111C8), (0x111CD, 0x111CD), (0x11238, 0x1123D), (0x114C6, 0x114C6),
    (0x115C1, 0x115C9), (0x11641, 0x11643), (0x12470, 0x12474), (0x16A6E, 0x16A6F), (0x16AF5, 0x16AF5), (0x16B37, 0x16B3B),
    (0x16B44, 0x16B44), (0x1BC9F, 0x1BC9F),
];

/// The character before `ix` in a subject that starts at `start`, as
/// cmark's delimiter scanners see it. The subject is a paragraph (whose
/// earlier lines end in a line feed), a heading or a table cell, so the
/// start reads as a line feed. When `skip_tildes`, `~` is transparent:
/// cmark-gfm's strikethrough extension registers it as an emphasis
/// character, and `scan_delims` steps over those (`parser->skip_chars`).
fn char_before(text: &str, start: usize, ix: usize, skip_tildes: bool) -> char {
    let bytes = text.as_bytes();
    let mut position = ix;
    while position > start {
        position -= 1;
        let byte = bytes[position];
        if byte & 0xC0 == 0x80 || (skip_tildes && byte == b'~') {
            continue;
        }
        return text[position..].chars().next().unwrap_or('\n');
    }
    '\n'
}

/// The character after a delimiter run ending at `ix`, as cmark's delimiter
/// scanners see it: a line feed at the end of the text, or at the end of a
/// table cell (cmark parses each cell as its own subject).
fn char_after(text: &str, ix: usize, skip_tildes: bool, table_cell: bool) -> char {
    let bytes = text.as_bytes();
    let mut position = ix;
    if skip_tildes {
        while position < bytes.len() && bytes[position] == b'~' {
            position += 1;
        }
    }
    match text[position..].chars().next() {
        None => '\n',
        Some('|') if table_cell => '\n',
        Some(c) => c,
    }
}

/// Whether a delimiter run of `c` from `ix` to `run_end`, in a subject that
/// starts at `start`, can open and can close: `scan_delims` for `*` and `_`,
/// the strikethrough extension's `cmark_inline_parser_scan_delimiters` for
/// `~`.
pub(crate) fn delimiter_run_flanking(
    text: &str,
    start: usize,
    ix: usize,
    run_end: usize,
    c: u8,
    table_cell: bool,
) -> (bool, bool) {
    let skip_tildes = c != b'~';
    let before = char_before(text, start, ix, skip_tildes);
    let after = char_after(text, run_end, skip_tildes, table_cell);
    let space_before = is_space(before);
    let space_after = is_space(after);
    let punct_before = is_punctuation(before);
    let punct_after = is_punctuation(after);
    let left_flanking = !space_after && (!punct_after || space_before || punct_before);
    let right_flanking = !space_before && (!punct_before || space_after || punct_after);
    if c == b'_' {
        (
            left_flanking && (!right_flanking || punct_before),
            right_flanking && (!left_flanking || punct_after),
        )
    } else {
        (left_flanking, right_flanking)
    }
}

/// The strikethrough extension scans at most 100 tildes at a time; the rest
/// of a longer run is scanned again as a run of its own.
pub(crate) const MAX_TILDE_RUN: usize = 100;

// MARK: Tables

/// `spacechar` in the table extension's scanners.
fn is_table_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | 0x0B | 0x0C)
}

/// The length of a line's content: up to its line ending or the end of
/// the text. cmark ends every line with a newline, so the end of the text
/// reads as one.
fn line_content_len(line: &[u8]) -> usize {
    memchr::memchr2(b'\n', b'\r', line).unwrap_or(line.len())
}

/// `scan_table_start`, then the delimiter row's alignments:
/// `[|]? table_marker ([|] table_marker)* [|]? spacechar* newline`, where a
/// marker is `spacechar* [:]? [-]+ [:]? spacechar*`. `line` starts at the
/// line's first non-space character. Unlike CommonMark tables elsewhere, a
/// row needs no pipe (`:-` is a one-column delimiter row) and a cell of
/// only `:` is not a marker.
pub(crate) fn table_delimiter_row(line: &[u8]) -> Option<Vec<Alignment>> {
    let len = line_content_len(line);
    let line = &line[..len];
    let mut p = 0;
    if line.first() == Some(&b'|') {
        p += 1;
    }
    let mut alignments = Vec::new();
    loop {
        while p < len && is_table_space(line[p]) {
            p += 1;
        }
        let left = p < len && line[p] == b':';
        if left {
            p += 1;
        }
        let dashes = p;
        while p < len && line[p] == b'-' {
            p += 1;
        }
        if p == dashes {
            return None;
        }
        let right = p < len && line[p] == b':';
        if right {
            p += 1;
        }
        while p < len && is_table_space(line[p]) {
            p += 1;
        }
        alignments.push(match (left, right) {
            (true, true) => Alignment::Center,
            (true, false) => Alignment::Left,
            (false, true) => Alignment::Right,
            (false, false) => Alignment::None,
        });
        if p < len && line[p] == b'|' {
            p += 1;
            // A trailing pipe, or the pipe before another marker.
            let mut q = p;
            while q < len && is_table_space(line[q]) {
                q += 1;
            }
            if q == len {
                return Some(alignments);
            }
            continue;
        }
        return if p == len { Some(alignments) } else { None };
    }
}

/// `scan_table_cell`: `(escaped_char | [^|\r\n])+`, the longest match, so a
/// pipe right after a backslash belongs to the cell.
fn table_cell(line: &[u8], offset: usize) -> usize {
    let mut end = offset;
    while end < line.len() {
        if line[end] == b'|' && !(end > offset && line[end - 1] == b'\\') {
            break;
        }
        end += 1;
    }
    end - offset
}

/// `scan_table_cell_end`: `[|] spacechar*`.
fn table_cell_end(line: &[u8], offset: usize) -> usize {
    if offset >= line.len() || line[offset] != b'|' {
        return 0;
    }
    let mut end = offset + 1;
    while end < line.len() && is_table_space(line[end]) {
        end += 1;
    }
    end - offset
}

/// The number of cells `row_from_string` finds in one line (from its first
/// non-space character), or 0 when it finds no row.
pub(crate) fn table_row_cells(line: &[u8]) -> usize {
    let len = line_content_len(line);
    let line = &line[..len];
    let mut offset = table_cell_end(line, 0);
    let mut cells = 0;
    while offset < len {
        let cell = table_cell(line, offset);
        let pipe = table_cell_end(line, offset + cell);
        if cell > 0 || pipe > 0 {
            cells += 1;
        }
        offset += cell + pipe;
        if pipe == 0 {
            // `scan_table_row_end`: only spaces may follow the last cell.
            while offset < len && is_table_space(line[offset]) {
                offset += 1;
            }
            if offset != len {
                return 0;
            }
        }
    }
    cells
}

// MARK: Links and reference definitions

/// `cmark_isspace`.
fn is_cmark_space_byte(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

/// `MAX_LINK_LABEL_LENGTH`.
const MAX_LINK_LABEL_LENGTH: usize = 1000;

/// `link_label`: a `[label]` (after `^` when `attribute`) at `p`. Returns the
/// label's contents (untrimmed) and the offset after the `]`.
fn link_label(input: &[u8], mut p: usize, attribute: bool) -> Option<(Range<usize>, usize)> {
    if attribute {
        if input.get(p) != Some(&b'^') {
            return None;
        }
        p += 1;
    }
    if input.get(p) != Some(&b'[') {
        return None;
    }
    p += 1;
    let start = p;
    let mut length = 0;
    loop {
        match input.get(p) {
            None | Some(b'[') => return None,
            Some(b']') => return Some((start..p, p + 1)),
            Some(b'\\') => {
                p += 1;
                length += 1;
                if input.get(p).is_some_and(u8::is_ascii_punctuation) {
                    p += 1;
                    length += 1;
                }
            }
            Some(_) => {
                p += 1;
                length += 1;
            }
        }
        if length > MAX_LINK_LABEL_LENGTH {
            return None;
        }
    }
}

/// `link_label` for a link label at `p`: its contents and the offset after
/// its `]`.
pub(crate) fn link_label_at(input: &[u8], p: usize) -> Option<(Range<usize>, usize)> {
    link_label(input, p, false)
}

/// `manual_scan_attribute_attributes`: the length of an inline attribute
/// span's attributes from `offset`, up to the `)` that closes them.
/// Parentheses nest (at most 32 deep) and backslashes escape punctuation;
/// unlike a link destination, spaces and line endings are allowed.
pub(crate) fn scan_attributes(input: &[u8], offset: usize) -> Option<usize> {
    let mut i = offset;
    let mut parens = 0;
    while i < input.len() {
        match input[i] {
            b'\\' if i + 1 < input.len() && input[i + 1].is_ascii_punctuation() => i += 2,
            b'(' => {
                parens += 1;
                i += 1;
                if parens > 32 {
                    return None;
                }
            }
            b')' => {
                if parens == 0 {
                    break;
                }
                parens -= 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    if i >= input.len() {
        return None;
    }
    Some(i - offset)
}

/// Whether a label has nothing but whitespace (`cmark_chunk_trim`, then a
/// test for emptiness).
pub(crate) fn label_is_blank(label: &[u8]) -> bool {
    label.iter().all(|&b| is_cmark_space_byte(b))
}

fn skip_spaces(input: &[u8], mut p: usize) -> usize {
    while p < input.len() && (input[p] == b' ' || input[p] == b'\t') {
        p += 1;
    }
    p
}

/// `skip_line_end`: an optional `\r`, an optional `\n`; true when one was
/// there or at the end of the input.
fn skip_line_end(input: &[u8], mut p: usize) -> (bool, usize) {
    let mut seen = false;
    if input.get(p) == Some(&b'\r') {
        p += 1;
        seen = true;
    }
    if input.get(p) == Some(&b'\n') {
        p += 1;
        seen = true;
    }
    (seen || p >= input.len(), p)
}

/// `spnl`: spaces, at most one line ending, spaces.
fn spnl(input: &[u8], p: usize) -> usize {
    let p = skip_spaces(input, p);
    let (seen, after) = skip_line_end(input, p);
    if seen {
        skip_spaces(input, after)
    } else {
        p
    }
}

/// `manual_scan_link_url`: a link destination at `offset`. Returns its
/// length and the destination (inside the brackets of the `<>` form). A
/// destination must end before the end of the input, may not start with a
/// space, and stops at the first space even inside parentheses, which need
/// not balance there; more than 32 open parentheses fail.
pub(crate) fn scan_link_url(input: &[u8], offset: usize) -> Option<(usize, Range<usize>)> {
    let mut i = offset;
    if input.get(i) == Some(&b'<') {
        i += 1;
        while i < input.len() {
            match input[i] {
                b'>' => {
                    i += 1;
                    break;
                }
                b'\\' => i += 2,
                b'\n' | b'<' => return None,
                _ => i += 1,
            }
        }
        if i >= input.len() {
            return None;
        }
        return Some((i - offset, offset + 1..i - 1));
    }
    let mut parens = 0;
    while i < input.len() {
        let c = input[i];
        if c == b'\\' && i + 1 < input.len() && input[i + 1].is_ascii_punctuation() {
            i += 2;
        } else if c == b'(' {
            parens += 1;
            i += 1;
            if parens > 32 {
                return None;
            }
        } else if c == b')' {
            if parens == 0 {
                break;
            }
            parens -= 1;
            i += 1;
        } else if is_cmark_space_byte(c) {
            if i == offset {
                return None;
            }
            break;
        } else {
            i += 1;
        }
    }
    if i >= input.len() {
        return None;
    }
    Some((i - offset, offset..i))
}

/// `scan_link_title`: `"..."`, `'...'` or `(...)`, where the delimiters
/// (and, in parentheses, `(`) may appear inside only after a backslash; the
/// longest match. Returns its length, 0 for none.
pub(crate) fn scan_link_title(input: &[u8], p: usize) -> usize {
    let (open, close) = match input.get(p) {
        Some(b'"') => (b'"', b'"'),
        Some(b'\'') => (b'\'', b'\''),
        Some(b'(') => (b'(', b')'),
        _ => return 0,
    };
    let mut end = 0;
    let mut i = p + 1;
    while i < input.len() {
        let c = input[i];
        let escaped = input[i - 1] == b'\\' && i - 1 > p;
        if c == close {
            end = i + 1;
            if !escaped {
                break;
            }
        } else if c == open && !escaped {
            break;
        }
        i += 1;
    }
    if end == 0 {
        0
    } else {
        end - p
    }
}

/// The key pulldown-cmark looks a label up by (its whitespace collapsed,
/// case-folded by `UniCase`), for a label found by `link_label`.
pub(crate) fn label_key(content: &[u8], label: Range<usize>) -> Option<LinkLabel<'static>> {
    label_key_with(content, label, &|_| Some(0)).map(UniCase::new)
}

/// `label_key` in text where `skip_prefix` measures the container prefix
/// after a line ending. `content[label.end]` is the `]` after the label.
pub(crate) fn label_key_with(
    content: &[u8],
    label: Range<usize>,
    skip_prefix: &dyn Fn(&[u8]) -> Option<usize>,
) -> Option<CowStr<'static>> {
    if label.len() > MAX_LINK_LABEL_LENGTH {
        return None;
    }
    let text = std::str::from_utf8(&content[label.start..label.end + 1]).ok()?;
    let (_, key) = scan_link_label_rest(text, skip_prefix, false)?;
    Some(CowStr::from(key.into_string()))
}

/// `houdini_unescape_ent` for an entity at `bytes[0]` (`&`): numeric
/// references take up to 8 digits, decimal or hexadecimal, where
/// CommonMark 0.31 allows 7 and 6. Returns its length and its value.
pub(crate) fn scan_entity(bytes: &[u8]) -> (usize, Option<CowStr<'static>>) {
    if bytes.get(1) != Some(&b'#') {
        return scanners::scan_entity(bytes);
    }
    let hex = matches!(bytes.get(2), Some(b'x' | b'X'));
    let digits_start = if hex { 3 } else { 2 };
    let mut codepoint: u32 = 0;
    let mut i = digits_start;
    while let Some(&b) = bytes.get(i) {
        let digit = match b {
            b'0'..=b'9' => u32::from(b - b'0'),
            b'a'..=b'f' | b'A'..=b'F' if hex => u32::from((b | 0x20) - b'a' + 10),
            _ => break,
        };
        codepoint = (codepoint * if hex { 16 } else { 10 } + digit).min(0x110000);
        i += 1;
    }
    let digits = i - digits_start;
    if !(1..=8).contains(&digits) || bytes.get(i) != Some(&b';') {
        return (0, None);
    }
    let value = match codepoint {
        0 | 0xD800..=0xDFFF | 0x110000.. => '\u{FFFD}',
        _ => char::from_u32(codepoint).unwrap_or('\u{FFFD}'),
    };
    (i + 1, Some(value.into()))
}

/// `houdini_unescape_html_f`: entities decoded.
fn decode_entities(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = String::with_capacity(text.len());
    let mut mark = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if let (n, Some(value)) = scan_entity(&bytes[i..]) {
                decoded.push_str(&text[mark..i]);
                decoded.push_str(&value);
                i += n;
                mark = i;
                continue;
            }
        }
        i += 1;
    }
    decoded.push_str(&text[mark..]);
    decoded
}

/// `cmark_strbuf_unescape`: a backslash before ASCII punctuation dropped.
fn unescape_backslashes(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    while let Some(c) = characters.next() {
        match characters.peek() {
            Some(&escaped) if c == '\\' && escaped.is_ascii_punctuation() => {
                result.push(escaped);
                characters.next();
            }
            _ => result.push(c),
        }
    }
    result
}

/// `cmark_strbuf_trim`'s characters.
fn trim_cmark_space(text: &str) -> &str {
    text.trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\x0B' | '\x0C' | '\r'))
}

/// `cmark_clean_url` and `cmark_clean_title`'s unescaping: entities first,
/// then backslash escapes in the result, so `\\&amp;` becomes `&`.
pub(crate) fn clean(text: &str) -> String {
    unescape_backslashes(&decode_entities(text))
}

/// A fenced code block's info string as cmark's `finalize` makes it:
/// entities decoded, then trimmed, then backslash escapes removed.
pub(crate) fn clean_info(text: &str) -> String {
    unescape_backslashes(trim_cmark_space(&decode_entities(text)))
}

/// `cmark_clean_url`: trimmed, then unescaped.
pub(crate) fn clean_url(text: &str) -> String {
    clean(trim_cmark_space(text))
}

/// An inline link's `(destination "title")` at `ix` (the `(`), as
/// `handle_close_bracket` scans it: spaces may include line endings,
/// `step` moves past a byte (and past the container prefix after a line
/// ending). Returns the offset after the `)`, the destination and the
/// title with its delimiters.
pub(crate) fn inline_link(
    bytes: &[u8],
    ix: usize,
    step: &dyn Fn(usize) -> usize,
) -> Option<(usize, Range<usize>, Option<Range<usize>>)> {
    if bytes.get(ix) != Some(&b'(') {
        return None;
    }
    let spaces = |mut p: usize| {
        while p < bytes.len() && is_cmark_space_byte(bytes[p]) {
            p = step(p);
        }
        p
    };
    let url_start = spaces(ix + 1);
    let (length, url) = scan_link_url(bytes, url_start)?;
    let url_end = url_start + length;
    let title_start = spaces(url_end);
    let title_end = if title_start == url_end {
        title_start
    } else {
        title_start + scan_link_title(bytes, title_start)
    };
    let end = spaces(title_end);
    if bytes.get(end) != Some(&b')') {
        return None;
    }
    let title = (title_end > title_start).then_some(title_start..title_end);
    Some((end + 1, url, title))
}

/// A definition at the start of a paragraph, as ranges in its content.
pub(crate) enum Definition {
    Link {
        label: Range<usize>,
        url: Range<usize>,
        title: Option<Range<usize>>,
    },
    Attributes {
        label: Range<usize>,
        attributes: Range<usize>,
    },
}

/// `cmark_parse_reference_inline`: a link reference definition at `p`.
fn reference_definition(input: &[u8], p: usize) -> Option<(usize, Definition)> {
    let (label, mut p) = link_label(input, p, false)?;
    if label_is_blank(&input[label.clone()]) || input.get(p) != Some(&b':') {
        return None;
    }
    p = spnl(input, p + 1);
    let (length, url) = scan_link_url(input, p)?;
    p += length;
    let before_title = p;
    p = spnl(input, p);
    let title_length = if p == before_title {
        0
    } else {
        scan_link_title(input, p)
    };
    let title = (title_length > 0).then(|| p..p + title_length);
    if title_length > 0 {
        p += title_length;
    } else {
        p = before_title;
    }
    let (ended, after) = skip_line_end(input, skip_spaces(input, p));
    let end = if ended {
        after
    } else if title_length > 0 {
        // Try again without the title. (cmark keeps the title it scanned.)
        let (ended, after) = skip_line_end(input, skip_spaces(input, before_title));
        if !ended {
            return None;
        }
        after
    } else {
        return None;
    };
    Some((end, Definition::Link { label, url, title }))
}

/// `cmark_parse_reference_attributes_inline`: `^[label]: attributes` at `p`
/// (swift-cmark's inline attributes).
fn attributes_definition(input: &[u8], p: usize) -> Option<(usize, Definition)> {
    let (label, mut p) = link_label(input, p, true)?;
    if label_is_blank(&input[label.clone()]) || input.get(p) != Some(&b':') {
        return None;
    }
    p = spnl(input, p + 1);
    let start = p;
    while p < input.len() && input[p] != b'\n' && input[p] != b'\r' {
        p += 1;
    }
    if p == start {
        return None;
    }
    let attributes = start..p;
    let (ended, end) = skip_line_end(input, skip_spaces(input, p));
    if !ended {
        return None;
    }
    Some((end, Definition::Attributes { label, attributes }))
}

/// `resolve_reference_link_definitions`: the definitions at the start of a
/// paragraph's content (each line from its first non-space character, with
/// its line ending). Returns how much of the content they take.
pub(crate) fn parse_definitions(content: &[u8]) -> (usize, Vec<Definition>) {
    let mut p = 0;
    let mut definitions = Vec::new();
    loop {
        let found = match content.get(p) {
            Some(b'[') => reference_definition(content, p),
            Some(b'^') if content.get(p + 1) == Some(&b'[') => attributes_definition(content, p),
            _ => None,
        };
        match found {
            Some((end, definition)) => {
                definitions.push(definition);
                p = end;
            }
            None => return (p, definitions),
        }
    }
}

// MARK: Task items

/// The whole line around `ix`, without its line ending.
pub(crate) fn line_around(bytes: &[u8], ix: usize) -> &[u8] {
    let start = bytes[..ix]
        .iter()
        .rposition(|&b| b == b'\n' || b == b'\r')
        .map_or(0, |p| p + 1);
    let end = ix + line_content_len(&bytes[ix..]);
    &bytes[start..end]
}

/// The tasklist extension's pattern, matched from the start of the line:
/// `spacechar* ("-"|"+"|"*"|[0-9]+.) spacechar+ ("[ ]"|"[x]") spacechar+`,
/// with `x` in either case. So a task needs text or a space after its box,
/// and an item after a block quote marker or another item's marker on the
/// same line is not a task.
pub(crate) fn is_task_line(line: &[u8]) -> bool {
    let mut position = 0;
    while position < line.len() && is_table_space(line[position]) {
        position += 1;
    }
    let Some(&marker) = line.get(position) else {
        return false;
    };
    let box_at = |mut position: usize| {
        let spaces = position;
        while position < line.len() && is_table_space(line[position]) {
            position += 1;
        }
        position > spaces
            && line.len() > position + 3
            && line[position] == b'['
            && matches!(line[position + 1], b' ' | b'x' | b'X')
            && line[position + 2] == b']'
            && is_table_space(line[position + 3])
    };
    if matches!(marker, b'-' | b'+' | b'*') {
        return box_at(position + 1);
    }
    if !marker.is_ascii_digit() {
        return false;
    }
    // `[0-9]+.`: the digits, then any one character; the digits may stop
    // anywhere, so every split is tried.
    let mut end = position;
    while end < line.len() && line[end].is_ascii_digit() {
        end += 1;
        if end < line.len() {
            let width = match line[end] {
                0x00..=0x7F => 1,
                0xC0..=0xDF => 2,
                0xE0..=0xEF => 3,
                _ => 4,
            };
            if box_at(end + width) {
                return true;
            }
        }
    }
    false
}

/// `open_tasklist_item`'s check: `[x]` or `[X]` anywhere on the line.
pub(crate) fn task_line_checked(line: &[u8]) -> bool {
    memchr::memmem::find(line, b"[x]").is_some() || memchr::memmem::find(line, b"[X]").is_some()
}

// MARK: HTML

/// `spacechar` in cmark's scanners.
fn is_html_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | 0x0B | 0x0C | b'\r' | b'\n')
}

/// `blocktagname` (CommonMark 0.29's list).
static BLOCK_TAG_NAMES: [&[u8]; 62] = [
    b"address",
    b"article",
    b"aside",
    b"base",
    b"basefont",
    b"blockquote",
    b"body",
    b"caption",
    b"center",
    b"col",
    b"colgroup",
    b"dd",
    b"details",
    b"dialog",
    b"dir",
    b"div",
    b"dl",
    b"dt",
    b"fieldset",
    b"figcaption",
    b"figure",
    b"footer",
    b"form",
    b"frame",
    b"frameset",
    b"h1",
    b"h2",
    b"h3",
    b"h4",
    b"h5",
    b"h6",
    b"head",
    b"header",
    b"hr",
    b"html",
    b"iframe",
    b"legend",
    b"li",
    b"link",
    b"main",
    b"menu",
    b"menuitem",
    b"nav",
    b"noframes",
    b"ol",
    b"optgroup",
    b"option",
    b"p",
    b"param",
    b"section",
    b"source",
    b"title",
    b"summary",
    b"table",
    b"tbody",
    b"td",
    b"tfoot",
    b"th",
    b"thead",
    b"tr",
    b"track",
    b"ul",
];

fn starts_with_ignoring_case(bytes: &[u8], prefix: &[u8]) -> bool {
    bytes.len() >= prefix.len() && bytes[..prefix.len()].eq_ignore_ascii_case(prefix)
}

/// `tagname`: `[A-Za-z][A-Za-z0-9-]*`. Returns its length.
fn tag_name(bytes: &[u8], p: usize) -> usize {
    if !bytes.get(p).is_some_and(u8::is_ascii_alphabetic) {
        return 0;
    }
    let mut end = p + 1;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'-') {
        end += 1;
    }
    end - p
}

/// `handle_pointy_brace`'s "skip" flags: once a scan for the end of a
/// comment, CDATA section, declaration or processing instruction has failed
/// in a subject, cmark stops looking for that kind. (A failed comment turns
/// off every `<!` form.)
#[derive(Default)]
pub(crate) struct InlineHtmlFlags {
    comment: bool,
    cdata: bool,
    declaration: bool,
    processing: bool,
}

/// Raw HTML after a `<` (at `bytes[ix - 1]`), as `handle_pointy_brace` finds
/// it: an open or closing tag, a comment, a processing instruction, a
/// declaration (`<!` and an uppercase name, then a space) or a CDATA
/// section. `bytes` ends where the block does; `skip_prefix` measures the
/// container prefix at the start of a line, which is not part of cmark's
/// subject. Returns the offset after the HTML.
pub(crate) fn inline_html(
    bytes: &[u8],
    ix: usize,
    flags: &mut InlineHtmlFlags,
    skip_prefix: &dyn Fn(&[u8]) -> usize,
) -> Option<usize> {
    // The offset after the byte at `i`, skipping a container prefix after a
    // line ending.
    let step = |i: usize| -> usize {
        match bytes[i] {
            b'\n' => i + 1 + skip_prefix(&bytes[i + 1..]),
            b'\r' if bytes.get(i + 1) != Some(&b'\n') => i + 1 + skip_prefix(&bytes[i + 1..]),
            _ => i + 1,
        }
    };
    match bytes.get(ix)? {
        b'!' => {
            if flags.comment {
                return None;
            }
            if bytes.get(ix + 1) == Some(&b'-') && bytes.get(ix + 2) == Some(&b'-') {
                match (bytes.get(ix + 3), bytes.get(ix + 4)) {
                    (Some(b'>'), _) => return Some(ix + 4),
                    (Some(b'-'), Some(b'>')) => return Some(ix + 5),
                    _ => {}
                }
                // `"--" ([^-]+ | "-" [^-] | "--" [^>])* "-->"`: after two
                // dashes, `>` ends the comment and anything else goes on.
                let mut dashes = 0;
                let mut i = ix + 3;
                while i < bytes.len() {
                    match bytes[i] {
                        b'-' if dashes < 2 => dashes += 1,
                        b'>' if dashes == 2 => return Some(i + 1),
                        _ => dashes = 0,
                    }
                    i = step(i);
                }
                flags.comment = true;
                return None;
            }
            if bytes.get(ix + 1) == Some(&b'[') {
                if flags.cdata || !bytes[ix + 2..].starts_with(b"CDATA[") {
                    return None;
                }
                // `([^]]+ | "]" [^]] | "]]" [^>])*`, then `]]>`.
                let mut brackets = 0;
                let mut i = ix + 8;
                while i < bytes.len() {
                    match bytes[i] {
                        b']' if brackets < 2 => brackets += 1,
                        b'>' if brackets == 2 => return Some(i + 1),
                        _ => brackets = 0,
                    }
                    i = step(i);
                }
                flags.cdata = true;
                return None;
            }
            if flags.declaration {
                return None;
            }
            // `[A-Z]+ spacechar+ [^>]*`, then `>`.
            let mut i = ix + 1;
            while bytes.get(i).is_some_and(u8::is_ascii_uppercase) {
                i += 1;
            }
            if i == ix + 1 || !bytes.get(i).is_some_and(|&b| is_html_space(b)) {
                return None;
            }
            while i < bytes.len() {
                if bytes[i] == b'>' {
                    return Some(i + 1);
                }
                i = step(i);
            }
            flags.declaration = true;
            None
        }
        b'?' => {
            if flags.processing {
                return None;
            }
            // `([^?>]+ | [?][^>] | [>])+`, then `?>`.
            let mut i = ix + 1;
            while i < bytes.len() {
                if bytes[i] == b'?' {
                    match bytes.get(i + 1) {
                        Some(b'>') => return Some(i + 2),
                        None => break,
                        Some(_) => i = step(i),
                    }
                }
                i = step(i);
            }
            flags.processing = true;
            None
        }
        _ => html_tag(bytes, ix, &step),
    }
}

/// `htmltag` (`opentag | closetag`) at `p`, just after a `<`; spaces include
/// line endings.
fn html_tag(bytes: &[u8], mut p: usize, step: &dyn Fn(usize) -> usize) -> Option<usize> {
    let skip_spaces = |mut p: usize| {
        while p < bytes.len() && is_html_space(bytes[p]) {
            p = step(p);
        }
        p
    };
    if bytes.get(p) == Some(&b'/') {
        let name = tag_name(bytes, p + 1);
        if name == 0 {
            return None;
        }
        p = skip_spaces(p + 1 + name);
        return (bytes.get(p) == Some(&b'>')).then_some(p + 1);
    }
    let name = tag_name(bytes, p);
    if name == 0 {
        return None;
    }
    p += name;
    loop {
        let name_start = skip_spaces(p);
        if name_start == p
            || !bytes
                .get(name_start)
                .is_some_and(|&b| b.is_ascii_alphabetic() || b == b'_' || b == b':')
        {
            break;
        }
        let mut q = name_start + 1;
        while q < bytes.len()
            && (bytes[q].is_ascii_alphanumeric() || matches!(bytes[q], b':' | b'.' | b'_' | b'-'))
        {
            q += 1;
        }
        p = q;
        let equals = skip_spaces(q);
        if bytes.get(equals) != Some(&b'=') {
            continue;
        }
        let value = skip_spaces(equals + 1);
        match bytes.get(value) {
            Some(&quote @ (b'\'' | b'"')) => {
                let mut i = value + 1;
                loop {
                    if i >= bytes.len() {
                        return None;
                    }
                    if bytes[i] == quote {
                        break;
                    }
                    i = step(i);
                }
                p = i + 1;
            }
            Some(_) => {
                let mut end = value;
                while end < bytes.len()
                    && !matches!(
                        bytes[end],
                        b' ' | b'\t'
                            | b'\r'
                            | b'\n'
                            | 0x0B
                            | 0x0C
                            | b'"'
                            | b'\''
                            | b'='
                            | b'<'
                            | b'>'
                            | b'`'
                            | 0
                    )
                {
                    end += 1;
                }
                if end == value {
                    return None;
                }
                p = end;
            }
            None => return None,
        }
    }
    p = skip_spaces(p);
    if bytes.get(p) == Some(&b'/') {
        p += 1;
    }
    (bytes.get(p) == Some(&b'>')).then_some(p + 1)
}

/// `scan_html_block_start` (kinds 1 to 6) and, when `kind_7` is allowed,
/// `scan_html_block_start_7`. `line` starts at the `<` and runs to the end
/// of the text. Kind 4 needs an uppercase letter after `<!` (CommonMark
/// 0.29), and the kind 6 names are 0.29's.
pub(crate) fn html_block_start(line: &[u8], kind_7: bool) -> Option<u8> {
    if line.first() != Some(&b'<') {
        return None;
    }
    let rest = &line[1..];
    let is_end = |b: Option<&u8>| b.map_or(true, |&b| is_html_space(b) || b == b'>');
    for name in [&b"script"[..], b"pre", b"textarea", b"style"] {
        if starts_with_ignoring_case(rest, name) && is_end(rest.get(name.len())) {
            return Some(1);
        }
    }
    if rest.starts_with(b"!--") {
        return Some(2);
    }
    if rest.starts_with(b"?") {
        return Some(3);
    }
    if rest.starts_with(b"!") && rest.get(1).is_some_and(u8::is_ascii_uppercase) {
        return Some(4);
    }
    if starts_with_ignoring_case(rest, b"![CDATA[") {
        return Some(5);
    }
    let slash = usize::from(rest.first() == Some(&b'/'));
    let name = tag_name(rest, slash);
    if name > 0 {
        let tag = &rest[slash..slash + name];
        if BLOCK_TAG_NAMES
            .iter()
            .any(|block| tag.eq_ignore_ascii_case(block))
        {
            match rest.get(slash + name) {
                None => return Some(6),
                Some(&b) if is_html_space(b) || b == b'>' => return Some(6),
                Some(b'/') if rest.get(slash + name + 1) == Some(&b'>') => return Some(6),
                _ => {}
            }
        }
    }
    if kind_7 {
        // `[<] (opentag | closetag) [\t\n\f ]* [\r\n]`, on one line.
        let len = line_content_len(line);
        let end = html_tag(&line[..len], 1, &|i| i + 1)?;
        if line[end..len]
            .iter()
            .all(|&b| matches!(b, b' ' | b'\t' | 0x0C))
        {
            return Some(7);
        }
    }
    None
}

/// What ends an HTML block of kinds 1 to 5, as `parse_html_block_type_1_to_5`
/// takes it. Kind 1 is matched by `html_block_ends`: any of the four end
/// tags, in any case.
pub(crate) const HTML_BLOCK_END_TAGS: [&str; 5] =
    ["</script|pre|textarea|style>", "-->", "?>", ">", "]]>"];

/// Whether `line` (a line of an HTML block of `kind`, from its first
/// non-space character) ends the block: `scan_html_block_end_1` to `_5`.
pub(crate) fn html_block_ends(kind: u8, line: &[u8]) -> bool {
    let line = &line[..memchr::memchr(b'\n', line).unwrap_or(line.len())];
    let contains = |needle: &[u8]| memchr::memmem::find(line, needle).is_some();
    match kind {
        1 => line.windows(2).enumerate().any(|(i, w)| {
            w == b"</"
                && [&b"script"[..], b"pre", b"textarea", b"style"]
                    .iter()
                    .any(|name| {
                        starts_with_ignoring_case(&line[i + 2..], name)
                            && line.get(i + 2 + name.len()) == Some(&b'>')
                    })
        }),
        2 => contains(b"-->"),
        3 => contains(b"?>"),
        4 => contains(b">"),
        5 => contains(b"]]>"),
        _ => false,
    }
}

/// `scan_close_code_fence`: at least `length` fence characters, then only
/// spaces and tabs (CommonMark 0.31 allows only spaces). Returns the length
/// of the fence and its spaces.
pub(crate) fn closing_code_fence(bytes: &[u8], fence: u8, length: usize) -> Option<usize> {
    if bytes.is_empty() {
        return Some(0);
    }
    let fences = bytes.iter().take_while(|&&b| b == fence).count();
    if fences < length {
        return None;
    }
    let spaces = bytes[fences..]
        .iter()
        .take_while(|&&b| b == b' ' || b == b'\t')
        .count();
    let end = fences + spaces;
    matches!(bytes.get(end), None | Some(b'\n' | b'\r')).then_some(end)
}

/// `chop_trailing_hashtags` on a line (without its line ending): the length
/// left once trailing whitespace goes, and then a closing sequence of `#`s
/// after a space or tab, and the whitespace before it.
pub(crate) fn chop_trailing_hashtags(line: &[u8]) -> usize {
    let rtrim = |mut len: usize| {
        while len > 0 && is_cmark_space_byte(line[len - 1]) {
            len -= 1;
        }
        len
    };
    let len = rtrim(line.len());
    let mut n = len;
    while n > 0 && line[n - 1] == b'#' {
        n -= 1;
    }
    if n != len && n > 0 && matches!(line[n - 1], b' ' | b'\t') {
        rtrim(n - 1)
    } else {
        len
    }
}

// MARK: Code spans

/// `MAXBACKTICKS`: a longer run never opens a code span.
const MAX_BACKTICKS: usize = 80;

/// `scan_to_closing_backticks`'s state in a subject: the last position of a
/// backtick run of each length, and whether a scan has reached the end.
/// Once one has, a run whose recorded position is not past the opener is
/// taken to have no closer, even when the record is only stale (the scan
/// for an earlier span overwrote it): "`` `e` `2`" leaves `2` as text.
pub(crate) struct Backticks {
    scanned: bool,
    positions: [usize; MAX_BACKTICKS + 1],
}

impl Backticks {
    pub(crate) fn new() -> Self {
        Backticks {
            scanned: false,
            positions: [0; MAX_BACKTICKS + 1],
        }
    }

    /// The `MaybeCode` node that closes a span opened at `open` with
    /// `count` backticks, if cmark finds one.
    pub(crate) fn find_closer(
        &mut self,
        tree: &Tree<Item>,
        open: TreeIndex,
        count: usize,
    ) -> Option<TreeIndex> {
        if count > MAX_BACKTICKS {
            return None;
        }
        if self.scanned && self.positions[count] <= open.get() {
            return None;
        }
        let mut scan = tree[open].next;
        while let Some(ix) = scan {
            if let ItemBody::MaybeCode(ticks, _) = tree[ix].item.body {
                if ticks <= MAX_BACKTICKS {
                    self.positions[ticks] = ix.get();
                }
                if ticks == count {
                    return Some(ix);
                }
            }
            scan = tree[ix].next;
        }
        self.scanned = true;
        None
    }
}

// MARK: Emphasis and strikethrough

/// A delimiter run on cmark's delimiter stack (`delimiter` in inlines.c).
/// Its characters are the tree nodes `start .. start + count`; an opener
/// gives up characters from its end, a closer from its start.
struct Delimiter {
    start: TreeIndex,
    count: usize,
    /// The run's original length, for the "multiple of 3" rule.
    length: usize,
    c: u8,
    can_open: bool,
    can_close: bool,
    /// The node before the run's first remaining character.
    before: Option<TreeIndex>,
    previous: Option<usize>,
    next: Option<usize>,
}

/// `process_emphasis` with cmark-gfm's strikethrough extension
/// (`strikethrough.c`'s `insert`), over the siblings from `tree.cur()`.
/// Resolves every `MaybeEmphasis` node into emphasis, strong emphasis,
/// strikethrough or text. A list of positions in the stack stands in for
/// cmark's subject offsets.
pub(crate) fn process_emphasis(tree: &mut Tree<Item>, text: &str) {
    let mut delimiters: Vec<Delimiter> = Vec::new();
    let mut before = None;
    let mut cursor = tree.cur();
    while let Some(ix) = cursor {
        if let ItemBody::MaybeEmphasis(count, can_open, can_close) = tree[ix].item.body {
            let previous = delimiters.len().checked_sub(1);
            if let Some(previous) = previous {
                delimiters[previous].next = Some(delimiters.len());
            }
            delimiters.push(Delimiter {
                start: ix,
                count,
                length: count,
                c: text.as_bytes()[tree[ix].item.start],
                can_open,
                can_close,
                before,
                previous,
                next: None,
            });
            let last = ix + (count - 1);
            before = Some(last);
            cursor = tree[last].next;
        } else {
            before = Some(ix);
            cursor = tree[ix].next;
        }
    }
    if delimiters.is_empty() {
        return;
    }

    // `openers_bottom`, by character and by the closer's length modulo 3.
    let mut openers_bottom = [[0usize; 3]; 3];
    let slot = |c: u8| match c {
        b'*' => 0,
        b'_' => 1,
        _ => 2,
    };
    let mut closer = Some(0);
    while let Some(closer_ix) = closer {
        let d = &delimiters[closer_ix];
        if !d.can_close {
            closer = d.next;
            continue;
        }
        let bottom = openers_bottom[slot(d.c)][d.length % 3];
        let mut opener = d.previous;
        let mut opener_found = false;
        while let Some(opener_ix) = opener {
            if opener_ix < bottom {
                break;
            }
            let o = &delimiters[opener_ix];
            if o.can_open
                && o.c == d.c
                && (!(d.can_open || o.can_close)
                    || d.length % 3 == 0
                    || (o.length + d.length) % 3 != 0)
            {
                opener_found = true;
                break;
            }
            opener = o.previous;
        }
        let old_closer = closer_ix;
        closer = match opener {
            Some(opener_ix) if opener_found => {
                if d.c == b'~' {
                    insert_strikethrough(tree, &mut delimiters, opener_ix, closer_ix)
                } else {
                    insert_emphasis(tree, &mut delimiters, opener_ix, closer_ix)
                }
            }
            _ => delimiters[closer_ix].next,
        };
        if !opener_found {
            let d = &delimiters[old_closer];
            openers_bottom[slot(d.c)][d.length % 3] = old_closer;
            if !d.can_open {
                remove_delimiter(&mut delimiters, old_closer);
            }
        }
    }

    // Whatever is left over is text.
    for d in &delimiters {
        for i in 0..d.count {
            tree[d.start + i].item.body = ItemBody::Text {
                backslash_escaped: false,
            };
        }
    }
}

fn remove_delimiter(delimiters: &mut [Delimiter], ix: usize) {
    let (previous, next) = (delimiters[ix].previous, delimiters[ix].next);
    if let Some(previous) = previous {
        delimiters[previous].next = next;
    }
    if let Some(next) = next {
        delimiters[next].previous = previous;
    }
}

/// Wraps the nodes between the opener's last `used` characters and the
/// closer's first `used` characters in a node of kind `body`. Returns the
/// new node.
fn wrap(
    tree: &mut Tree<Item>,
    delimiters: &mut [Delimiter],
    opener_ix: usize,
    closer_ix: usize,
    used: usize,
    body: ItemBody,
) -> TreeIndex {
    let (o_start, o_count) = (delimiters[opener_ix].start, delimiters[opener_ix].count);
    let (c_start, c_count) = (delimiters[closer_ix].start, delimiters[closer_ix].count);
    let root = o_start + (o_count - used);
    let first = root + used;
    let child = if first == c_start {
        None
    } else {
        if let Some(last) = delimiters[closer_ix].before {
            tree[last].next = None;
        }
        Some(first)
    };
    let closer_last = c_start + (used - 1);
    tree[root].item.body = body;
    tree[root].item.end = tree[closer_last].item.end;
    tree[root].child = child;
    tree[root].next = if c_count > used {
        Some(closer_last + 1)
    } else {
        tree[c_start + (c_count - 1)].next
    };
    delimiters[opener_ix].count -= used;
    delimiters[closer_ix].count -= used;
    delimiters[closer_ix].start = closer_last + 1;
    delimiters[closer_ix].before = Some(root);
    if delimiters[closer_ix].count == 0 {
        // The delimiter after the closer now follows the new node.
        let old_last = c_start + (c_count - 1);
        if let Some(next) = delimiters[closer_ix].next {
            if delimiters[next].before == Some(old_last) {
                delimiters[next].before = Some(root);
            }
        }
    }
    root
}

/// `S_insert_emph`.
fn insert_emphasis(
    tree: &mut Tree<Item>,
    delimiters: &mut [Delimiter],
    opener_ix: usize,
    closer_ix: usize,
) -> Option<usize> {
    let used = if delimiters[closer_ix].count >= 2 && delimiters[opener_ix].count >= 2 {
        2
    } else {
        1
    };
    // Free the delimiters between opener and closer.
    let mut between = delimiters[closer_ix].previous;
    while let Some(ix) = between {
        if ix == opener_ix {
            break;
        }
        between = delimiters[ix].previous;
        remove_delimiter(delimiters, ix);
    }
    let body = if used == 1 {
        ItemBody::Emphasis
    } else {
        ItemBody::Strong
    };
    wrap(tree, delimiters, opener_ix, closer_ix, used, body);
    if delimiters[opener_ix].count == 0 {
        remove_delimiter(delimiters, opener_ix);
    }
    if delimiters[closer_ix].count == 0 {
        let next = delimiters[closer_ix].next;
        remove_delimiter(delimiters, closer_ix);
        next
    } else {
        Some(closer_ix)
    }
}

/// The strikethrough extension's `insert`: runs of the same length make
/// strikethrough; either way every delimiter from the opener to the closer
/// is removed.
fn insert_strikethrough(
    tree: &mut Tree<Item>,
    delimiters: &mut [Delimiter],
    opener_ix: usize,
    closer_ix: usize,
) -> Option<usize> {
    let next = delimiters[closer_ix].next;
    let count = delimiters[opener_ix].count;
    if count == delimiters[closer_ix].count {
        wrap(
            tree,
            delimiters,
            opener_ix,
            closer_ix,
            count,
            ItemBody::Strikethrough,
        );
    }
    let mut ix = Some(closer_ix);
    while let Some(current) = ix {
        let previous = delimiters[current].previous;
        remove_delimiter(delimiters, current);
        if current == opener_ix {
            break;
        }
        ix = previous;
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn punctuation_leaves_out_symbols() {
        assert!(is_punctuation('!'));
        assert!(is_punctuation('~'));
        assert!(is_punctuation('\u{00A1}'));
        assert!(is_punctuation('\u{2014}'));
        assert!(is_punctuation('\u{1BC9F}'));
        assert!(!is_punctuation('a'));
        assert!(!is_punctuation('\u{2705}'));
        assert!(!is_punctuation('\u{1F680}'));
        assert!(!is_punctuation('\u{00A2}'));
        assert!(!is_punctuation('\u{20AC}'));
    }

    #[test]
    fn space_is_cmark_space() {
        assert!(is_space('\u{00A0}'));
        assert!(is_space('\u{3000}'));
        assert!(!is_space('\u{000B}'));
        assert!(!is_space('\u{2028}'));
    }
}
