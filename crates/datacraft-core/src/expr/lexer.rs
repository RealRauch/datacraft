//! Expression lexer (step 15.1, item H41).
//!
//! Tokens are numbers (`42`, `2.5` — no exponents), strings in double or
//! single quotes with doubling as the escape (`"say ""hi"""`), date
//! literals `#YYYY-MM-DD[ HH:MM]#`, identifiers (bare or `[bracketed]`,
//! brackets allow any character but `]`), the documented keywords
//! (case-insensitive) and the punctuation operators.

use super::ExprError;

/// One lexical token.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    /// Integer literal.
    Integer(i64),
    /// Float literal (fraction present).
    Float(f64),
    /// String literal with escapes resolved.
    Text(String),
    /// Date literal content (validated by the parser).
    Date(String),
    /// Bare identifier or keyword-shaped name.
    Ident(String),
    /// `[Bracketed]` identifier (may contain spaces and punctuation).
    Bracket(String),
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `\`
    Backslash,
    /// `^`
    Caret,
    /// `&`
    Amp,
    /// `=`
    Eq,
    /// `<>`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `,`
    Comma,
    /// `.` (qualified-name separator)
    Dot,
    /// `And`
    And,
    /// `Or`
    Or,
    /// `Not`
    Not,
    /// `Like`
    Like,
    /// `Is`
    Is,
    /// `Null`
    NullKw,
    /// `True`
    True,
    /// `False`
    False,
    /// `Between`
    Between,
    /// `In`
    In,
    /// `Mod`
    Mod,
}

/// A token with its byte offset (error positions).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned {
    /// The token itself.
    pub(crate) token: Token,
    /// Byte offset where the token starts.
    pub(crate) start: usize,
}

/// Tokenizes the source; lexical failures carry the offending position.
pub(crate) fn lex(source: &str) -> Result<Vec<Spanned>, ExprError> {
    let chars: Vec<(usize, char)> = source.char_indices().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let (start, c) = chars[i];
        i += 1;
        if c.is_whitespace() {
            continue;
        }
        if c.is_ascii_digit() {
            let number = scan_number(&chars, &mut i, start, c)?;
            tokens.push(number);
            continue;
        }
        let token = match c {
            '"' | '\'' => scan_string(&chars, &mut i, c, start)?,
            '#' => Token::Date(scan_delimited(&chars, &mut i, '#', start)?),
            '[' => Token::Bracket(scan_delimited(&chars, &mut i, ']', start)?),
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '\\' => Token::Backslash,
            '^' => Token::Caret,
            '&' => Token::Amp,
            '(' => Token::LParen,
            ')' => Token::RParen,
            ',' => Token::Comma,
            '.' => Token::Dot,
            '<' => match chars.get(i).map(|&(_, c)| c) {
                Some('=') => {
                    i += 1;
                    Token::Le
                }
                Some('>') => {
                    i += 1;
                    Token::Ne
                }
                _ => Token::Lt,
            },
            '>' => {
                if chars.get(i).map(|&(_, c)| c) == Some('=') {
                    i += 1;
                    Token::Ge
                } else {
                    Token::Gt
                }
            }
            '=' => Token::Eq,
            c if c.is_ascii_alphabetic() || c == '_' => {
                let word: String = std::iter::once(c)
                    .chain(
                        chars[i..]
                            .iter()
                            .take_while(|&&(_, c)| c.is_ascii_alphanumeric() || c == '_')
                            .map(|&(_, c)| c),
                    )
                    .collect();
                i += word.chars().count() - 1;
                keyword_or_ident(&word)
            }
            other => {
                return Err(ExprError::Lex {
                    position: start,
                    detail: format!("unexpected character {other:?}"),
                });
            }
        };
        tokens.push(Spanned { token, start });
    }
    Ok(tokens)
}

/// Scans a number: digits with an optional fraction (documented: no
/// exponents; numbers start with a digit). `first` is the already
/// consumed leading digit.
fn scan_number(
    chars: &[(usize, char)],
    i: &mut usize,
    start: usize,
    first: char,
) -> Result<Spanned, ExprError> {
    let mut digits = String::from(first);
    while matches!(chars.get(*i), Some((_, '0'..='9'))) {
        digits.push(chars[*i].1);
        *i += 1;
    }
    let is_float = matches!(chars.get(*i), Some((_, '.')))
        && matches!(chars.get(*i + 1), Some((_, '0'..='9')));
    if is_float {
        digits.push('.');
        *i += 1;
        while matches!(chars.get(*i), Some((_, '0'..='9'))) {
            digits.push(chars[*i].1);
            *i += 1;
        }
    }
    let token = if is_float {
        // The digits are `[0-9]+(\.[0-9]+)?`, so the parse cannot fail —
        // f64 saturates to infinity instead. The house pattern (AGENTS §6)
        // still answers a precise error instead of expecting.
        match digits.parse::<f64>() {
            Ok(value) => Token::Float(value),
            Err(_) => {
                return Err(ExprError::Lex {
                    position: start,
                    detail: "float literal is not representable".to_owned(),
                });
            }
        }
    } else {
        match digits.parse::<i64>() {
            Ok(value) => Token::Integer(value),
            Err(_) => {
                return Err(ExprError::Lex {
                    position: start,
                    detail: "integer literal exceeds 64 bits".to_owned(),
                });
            }
        }
    };
    Ok(Spanned { token, start })
}

/// Scans a quoted string; the quote character is escaped by doubling.
fn scan_string(
    chars: &[(usize, char)],
    i: &mut usize,
    quote: char,
    start: usize,
) -> Result<Token, ExprError> {
    let mut text = String::new();
    loop {
        match chars.get(*i) {
            Some((_, c)) if *c == quote => {
                *i += 1;
                if chars.get(*i).map(|&(_, c)| c) == Some(quote) {
                    text.push(quote);
                    *i += 1;
                } else {
                    return Ok(Token::Text(text));
                }
            }
            Some((_, c)) => {
                text.push(*c);
                *i += 1;
            }
            None => {
                return Err(ExprError::Lex {
                    position: start,
                    detail: "unterminated string".to_owned(),
                });
            }
        }
    }
}

/// Scans until the closing delimiter (`#…#`, `[…]`).
fn scan_delimited(
    chars: &[(usize, char)],
    i: &mut usize,
    delimiter: char,
    start: usize,
) -> Result<String, ExprError> {
    let mut text = String::new();
    loop {
        match chars.get(*i) {
            Some((_, c)) if *c == delimiter => {
                *i += 1;
                return Ok(text);
            }
            Some((_, c)) => {
                text.push(*c);
                *i += 1;
            }
            None => {
                return Err(ExprError::Lex {
                    position: start,
                    detail: format!("missing closing {delimiter:?}"),
                });
            }
        }
    }
}

/// Lower-case keyword table; everything else stays an identifier.
fn keyword_or_ident(word: &str) -> Token {
    match word.to_lowercase().as_str() {
        "and" => Token::And,
        "or" => Token::Or,
        "not" => Token::Not,
        "like" => Token::Like,
        "is" => Token::Is,
        "null" => Token::NullKw,
        "true" => Token::True,
        "false" => Token::False,
        "between" => Token::Between,
        "in" => Token::In,
        "mod" => Token::Mod,
        _ => Token::Ident(word.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<Token> {
        lex(source)
            .expect("source must lex")
            .into_iter()
            .map(|spanned| spanned.token)
            .collect()
    }

    #[test]
    fn operators_lex_in_longest_match_order() {
        assert_eq!(
            kinds("< <= <> > >= = + - * / \\ ^ & ( ) , ."),
            vec![
                Token::Lt,
                Token::Le,
                Token::Ne,
                Token::Gt,
                Token::Ge,
                Token::Eq,
                Token::Plus,
                Token::Minus,
                Token::Star,
                Token::Slash,
                Token::Backslash,
                Token::Caret,
                Token::Amp,
                Token::LParen,
                Token::RParen,
                Token::Comma,
                Token::Dot,
            ]
        );
    }

    #[test]
    fn literals_and_keywords_lex_case_insensitively() {
        assert_eq!(
            kinds("42 2.5 \"a\"\"b\" 'c' #2025-01-05# [Order Details] between TRUE null Mod"),
            vec![
                Token::Integer(42),
                Token::Float(2.5),
                Token::Text("a\"b".into()),
                Token::Text("c".into()),
                Token::Date("2025-01-05".into()),
                Token::Bracket("Order Details".into()),
                Token::Between,
                Token::True,
                Token::NullKw,
                Token::Mod,
            ]
        );
    }

    #[test]
    fn unknown_characters_are_lexical_errors() {
        let error = lex("1 @ 2").expect_err("@ must fail");
        assert!(matches!(error, ExprError::Lex { position: 2, .. }));
    }

    #[test]
    fn unterminated_strings_and_brackets_fail() {
        assert!(lex("\"open").is_err());
        assert!(lex("'open").is_err());
        assert!(lex("#2025-01-05").is_err());
        assert!(lex("[open").is_err());
    }

    /// An integer literal that no longer fits i64 fails precisely at the
    /// lex stage.
    #[test]
    fn oversized_integer_literals_fail_precisely() {
        let error = lex("9223372036854775808").expect_err("2^63 does not fit i64");
        assert!(error.to_string().contains("exceeds 64 bits"), "{error}");
    }
}
