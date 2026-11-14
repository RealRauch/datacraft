//! `Like` matching with Access wildcards (step 15.1, item H41).
//!
//! `*` matches any run of characters, `?` exactly one, `#` exactly one
//! digit, `[…]` a character class with `!` negation and `-` ranges.
//! Wildcards inside a class are literal; an unclosed `[` is a literal
//! bracket. Matching is case-insensitive (Access `Option Compare
//! Database`).

/// Case-insensitive Access-wildcard match.
pub fn matches(text: &str, pattern: &str) -> bool {
    let text: Vec<char> = text.chars().map(fold).collect();
    let pattern: Vec<char> = pattern.chars().map(fold).collect();
    match_here(&text, &pattern)
}

/// Simple case fold (first lowercase char; umlauts included).
fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

fn match_here(text: &[char], pattern: &[char]) -> bool {
    match pattern.split_first() {
        None => text.is_empty(),
        Some(('*', rest)) => (0..=text.len()).any(|skip| match_here(&text[skip..], rest)),
        Some(('?', rest)) => !text.is_empty() && match_here(&text[1..], rest),
        Some(('#', rest)) => {
            !text.is_empty() && text[0].is_ascii_digit() && match_here(&text[1..], rest)
        }
        Some(('[', _)) => match split_class(pattern) {
            Some((negated, class, rest)) => {
                !text.is_empty()
                    && class.contains(text[0]) != negated
                    && match_here(&text[1..], rest)
            }
            None => {
                // Unclosed bracket: literal.
                !text.is_empty() && text[0] == '[' && match_here(&text[1..], &pattern[1..])
            }
        },
        Some((first, rest)) => {
            !text.is_empty() && text[0] == *first && match_here(&text[1..], rest)
        }
    }
}

/// A character class: singles plus inclusive ranges.
#[derive(Debug, PartialEq)]
struct CharClass {
    singles: Vec<char>,
    ranges: Vec<(char, char)>,
}

impl CharClass {
    fn contains(&self, c: char) -> bool {
        self.singles.contains(&c)
            || self
                .ranges
                .iter()
                .any(|(low, high)| c >= *low && c <= *high)
    }
}

/// Splits a leading `[…]` class into its members and the remaining
/// pattern; `None` when the class never closes.
fn split_class(pattern: &[char]) -> Option<(bool, CharClass, &[char])> {
    let mut index = 1;
    let negated = pattern.get(index) == Some(&'!');
    if negated {
        index += 1;
    }
    let mut class = CharClass {
        singles: Vec::new(),
        ranges: Vec::new(),
    };
    loop {
        let &c = pattern.get(index)?;
        if c == ']' {
            return Some((negated, class, &pattern[index + 1..]));
        }
        if pattern.get(index + 1) == Some(&'-')
            && let Some(&high) = pattern.get(index + 2)
            && high != ']'
        {
            class.ranges.push((c, high));
            index += 3;
        } else {
            class.singles.push(c);
            index += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards_match_the_documented_matrix() {
        assert!(matches("K-42", "K-*"));
        assert!(matches("k-42", "K-*"));
        assert!(matches("abc", "a?c"));
        assert!(!matches("ab", "a?c"));
        assert!(matches("a1", "a#"));
        assert!(!matches("ab", "a#"));
        assert!(matches("m", "[m-p]"));
        assert!(!matches("z", "[m-p]"));
        assert!(matches("z", "[!m-p]"));
        assert!(!matches("m", "[!m-p]"));
        assert!(matches("a*b", "a[*]b"));
        assert!(!matches("axb", "a[*]b"));
    }

    #[test]
    fn umlauts_fold_case_insensitively() {
        assert!(matches("Ärger", "är*"));
        assert!(matches("Übung", "[ÜY]bung"));
    }

    #[test]
    fn unclosed_brackets_stay_literal() {
        assert!(matches("a[b", "a[b"));
        assert!(!matches("ab", "a[b"));
    }

    #[test]
    fn empty_operands_behave_like_vba() {
        assert!(matches("", "*"));
        assert!(matches("", ""));
        assert!(!matches("a", ""));
    }
}
