//! `CREATE TABLE` text parsing for the SQLite catalog reader.
//!
//! `sqlite_master.sql` stores the original DDL text. Deciding whether a
//! table's integer primary key is an `AUTOINCREMENT` key must look at
//! that key column's own definition only — a mention of the word
//! anywhere else (another column's default, a quoted column name)
//! must not turn the key into an AutoNumber (step 42.1, item L13).

/// True when `column`'s own definition in `create_sql` carries the
/// `AUTOINCREMENT` keyword as a standalone token with quoted regions
/// removed. Malformed or empty texts answer `false`.
pub(crate) fn pk_is_autoincrement(create_sql: &str, column: &str) -> bool {
    column_segments(create_sql)
        .into_iter()
        .any(|segment| match segment_parts(segment) {
            Some((name, rest)) => {
                names_equal(name, column)
                    && contains_word(&strip_quoted(rest).to_ascii_uppercase(), "AUTOINCREMENT")
            }
            None => false,
        })
}

/// Column-definition segments of a `CREATE TABLE` statement's first
/// top-level parenthesized group, split on top-level commas. Quoted
/// regions keep their content, so a comma inside a literal never
/// splits. Returns an empty list when no well-formed group exists.
fn column_segments(create_sql: &str) -> Vec<&str> {
    let bytes = create_sql.as_bytes();
    let mut segments = Vec::new();
    let mut quote: Option<u8> = None; // closing byte (']' for '[')
    let mut depth = 0usize;
    let mut segment_start: Option<usize> = None;
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(close) = quote {
            if byte == close {
                if close != b']' && bytes.get(index + 1) == Some(&close) {
                    index += 1; // a doubled quote stays inside the region
                } else {
                    quote = None;
                }
            }
            index += 1;
            continue;
        }
        match byte {
            b'"' | b'\'' | b'`' => quote = Some(byte),
            b'[' => quote = Some(b']'),
            b'(' => {
                depth += 1;
                if depth == 1 {
                    segment_start = Some(index + 1);
                }
            }
            b')' => {
                if depth == 1 {
                    if let Some(start) = segment_start {
                        push_segment(create_sql, start, index, &mut segments);
                    }
                    return segments;
                }
                depth = depth.saturating_sub(1);
            }
            b',' if depth == 1 => {
                if let Some(start) = segment_start {
                    push_segment(create_sql, start, index, &mut segments);
                    segment_start = Some(index + 1);
                }
            }
            _ => {}
        }
        index += 1;
    }
    // Unterminated group — not a usable column list.
    Vec::new()
}

fn push_segment<'a>(sql: &'a str, start: usize, end: usize, segments: &mut Vec<&'a str>) {
    let segment = sql[start..end].trim();
    if !segment.is_empty() {
        segments.push(segment);
    }
}

/// Splits a column-definition segment into its raw identifier (quote
/// doubling intact) and the definition text after the name. Bare
/// identifiers end at the first whitespace or type parenthesis;
/// unterminated quoted names answer `None`.
fn segment_parts(segment: &str) -> Option<(&str, &str)> {
    let text = segment.trim_start();
    let (close, doubled) = match text.as_bytes().first()? {
        b'"' => (b'"', true),
        b'\'' => (b'\'', true),
        b'`' => (b'`', true),
        b'[' => (b']', false),
        _ => {
            let end = text
                .find(|c: char| c.is_whitespace() || c == '(')
                .unwrap_or(text.len());
            return Some((&text[..end], &text[end..]));
        }
    };
    let inner = &text[1..];
    let bytes = inner.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == close {
            if doubled && bytes.get(index + 1) == Some(&close) {
                index += 1; // doubled quote inside the name
            } else {
                return Some((&inner[..index], &inner[index + 1..]));
            }
        }
        index += 1;
    }
    None
}

/// Removes every quoted region (`"ident"`, `'literal'`, `` `ident` ``,
/// `[ident]`) and keeps the text between them; an unterminated quote
/// drops the rest of the text. Token searches run over this text, so
/// words inside names or literals never match.
fn strip_quoted(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut run_start = 0usize;
    let mut quote: Option<u8> = None;
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(close) = quote {
            if byte == close {
                if close != b']' && bytes.get(index + 1) == Some(&close) {
                    index += 1;
                } else {
                    quote = None;
                    run_start = index + 1;
                }
            }
            index += 1;
            continue;
        }
        match byte {
            b'"' | b'\'' | b'`' => {
                out.push_str(&text[run_start..index]);
                run_start = index + 1;
                quote = Some(byte);
            }
            b'[' => {
                out.push_str(&text[run_start..index]);
                run_start = index + 1;
                quote = Some(b']');
            }
            _ => {}
        }
        index += 1;
    }
    if quote.is_none() {
        out.push_str(&text[run_start..]);
    }
    out
}

/// Word-boundary token search: identifier characters around an
/// occurrence keep it from matching (`NOAUTOINCREMENT` does not
/// contain the `AUTOINCREMENT` token).
fn contains_word(text: &str, token: &str) -> bool {
    let bytes = text.as_bytes();
    let boundary = |byte: u8| !(byte.is_ascii_alphanumeric() || byte == b'_');
    text.match_indices(token).any(|(at, found)| {
        let before = at == 0 || boundary(bytes[at - 1]);
        let end = at + found.len();
        let after = end == bytes.len() || boundary(bytes[end]);
        before && after
    })
}

/// Identifier comparison: ASCII case-insensitive (SQLite's rule) with
/// doubled quote escapes collapsed on both sides.
fn names_equal(a: &str, b: &str) -> bool {
    let collapse = |name: &str| {
        name.replace("\"\"", "\"")
            .replace("''", "'")
            .replace("``", "`")
    };
    collapse(a).eq_ignore_ascii_case(&collapse(b))
}

#[cfg(test)]
#[path = "ddl_tests.rs"]
mod tests;
