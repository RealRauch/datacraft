use super::*;

#[test]
fn detects_autoincrement_on_the_key_column_definition() {
    // Double-quoted identifier, exactly the writer's shape.
    assert!(pk_is_autoincrement(
        "CREATE TABLE \"items\" (\"id\" INTEGER PRIMARY KEY AUTOINCREMENT, \"note\" TEXT)",
        "id"
    ));
    // Bare identifiers, mixed case and extra whitespace.
    assert!(pk_is_autoincrement(
        "CREATE TABLE Items (ID integer primary key autoincrement, note TEXT)",
        "id"
    ));
    // Bracket- and backtick-quoted names.
    assert!(pk_is_autoincrement(
        "CREATE TABLE t ([id] INTEGER PRIMARY KEY AUTOINCREMENT)",
        "id"
    ));
    assert!(pk_is_autoincrement(
        "CREATE TABLE t (`id` INTEGER PRIMARY KEY AUTOINCREMENT)",
        "id"
    ));
    // Doubled quote inside the key column's name.
    assert!(pk_is_autoincrement(
        "CREATE TABLE t (\"we\"\"ird\" INTEGER PRIMARY KEY AUTOINCREMENT)",
        "we\"ird"
    ));
    // Nested parentheses in an earlier column never break the split.
    assert!(pk_is_autoincrement(
        "CREATE TABLE t (ts DATETIME DEFAULT (datetime('now, o clock')), \
         id INTEGER PRIMARY KEY AUTOINCREMENT)",
        "id"
    ));
    // A trailing table option after the group is ignored.
    assert!(pk_is_autoincrement(
        "CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT) WITHOUT ROWID",
        "id"
    ));
}

#[test]
fn a_word_mention_outside_the_key_column_is_not_an_autoincrement() {
    // Another column's default string.
    assert!(!pk_is_autoincrement(
        "CREATE TABLE t (id INTEGER PRIMARY KEY, note TEXT DEFAULT 'AUTOINCREMENT')",
        "id"
    ));
    // A column actually named with the word (quoted, so no token).
    assert!(!pk_is_autoincrement(
        "CREATE TABLE t (\"AUTOINCREMENT\" INTEGER PRIMARY KEY, note TEXT)",
        "AUTOINCREMENT"
    ));
    // The table-level primary key clause is no column definition.
    assert!(!pk_is_autoincrement(
        "CREATE TABLE t (id INTEGER, PRIMARY KEY (id), note TEXT DEFAULT 'AUTOINCREMENT')",
        "id"
    ));
    // Asking about a different column than the keyed one answers false.
    assert!(!pk_is_autoincrement(
        "CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT)",
        "note"
    ));
}

#[test]
fn token_search_is_word_bounded() {
    assert!(contains_word("x AUTOINCREMENT y", "AUTOINCREMENT"));
    assert!(contains_word("AUTOINCREMENT", "AUTOINCREMENT"));
    assert!(!contains_word("NOAUTOINCREMENTHERE", "AUTOINCREMENT"));
    assert!(!contains_word("autoincrement_x", "AUTOINCREMENT"));
    assert!(!contains_word("x_autoincrement", "AUTOINCREMENT"));
    assert!(!contains_word("", "AUTOINCREMENT"));
}

#[test]
fn quoted_regions_are_stripped_and_unterminated_quotes_drop_the_tail() {
    assert_eq!(strip_quoted("a\"b\"c"), "ac");
    assert_eq!(strip_quoted("'it''s' x"), " x");
    assert_eq!(strip_quoted("[a]b"), "b");
    assert_eq!(strip_quoted("`a`b"), "b");
    assert_eq!(strip_quoted("unterminated\"xyz"), "unterminated");
    // Multi-byte text outside quotes survives.
    assert_eq!(strip_quoted("möhre"), "möhre");
}

#[test]
fn malformed_create_texts_answer_false() {
    assert!(!pk_is_autoincrement("", "id"));
    assert!(!pk_is_autoincrement("CREATE TABLE t", "id"));
    assert!(!pk_is_autoincrement("CREATE TABLE t ()", "id"));
    // Truncated after the keyword: the group never closes.
    assert!(!pk_is_autoincrement(
        "CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT",
        "id"
    ));
}
