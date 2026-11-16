use cartan_syntax::prompt::{completeness, highlight, Completeness, HighlightClass};

#[test]
fn prompt_text_is_complete_waiting_or_invalid() {
    use Completeness::*;
    for (text, expected) in [
        ("", Complete),
        ("x = 1\n", Complete),
        ("1 + 2", Complete),
        ("main =", Incomplete),
        ("main = [1, 2", Incomplete),
        ("f(x) = {\n  y := x\n", Incomplete),
        // An unfinished call waits for its `)`; it is not a refused call head.
        ("x = f(1", Incomplete),
        ("x = f(", Incomplete),
        ("x = = 2", Invalid),
        ("x = [1, 2)", Invalid),
        ("x = 1 2", Invalid),
        // A bad line stays invalid when a later line, read after it, waits.
        ("x = 1 2\ny = (", Invalid),
        // Documentation waits for its closing delimiter, wherever it stands.
        ("\"\"\"", Incomplete),
        ("\"\"\"\nThe radius", Incomplete),
        ("\"\"\"The radius\nof a face", Incomplete),
        ("x = 1\n\"\"\"\nThe radius\n", Incomplete),
        ("\"\"\"The radius.\"\"\"\nr = 1\n", Complete),
    ] {
        assert_eq!(completeness(text), expected, "{text:?}");
    }
}

#[test]
fn highlighting_colors_partial_text_from_the_grammar_lexis() {
    use HighlightClass::*;
    let text = "while u < f(1.5e3, \"a\\\"b\", none) # tail\nx := [true";
    let classes: Vec<(&str, HighlightClass)> = highlight(text)
        .iter()
        .map(|h| (&text[h.span.start..h.span.end], h.class))
        .collect();
    assert_eq!(
        classes,
        [
            ("while", Keyword),
            ("u", Name),
            ("<", Operator),
            ("f", Name),
            ("(", Punctuation),
            ("1.5e3", Number),
            (",", Punctuation),
            ("\"a\\\"b\"", String),
            (",", Punctuation),
            ("none", Constant),
            (")", Punctuation),
            ("# tail", Comment),
            ("x", Name),
            (":=", Operator),
            ("[", Punctuation),
            ("true", Constant),
        ]
    );
    let range = highlight("0..10");
    assert_eq!(
        range.iter().map(|h| h.class).collect::<Vec<_>>(),
        [Number, Operator, Number]
    );
    // A lock clause's words are keywords, though only the clause spells them.
    let text = "(locked when c)";
    assert_eq!(
        highlight(text)
            .iter()
            .map(|h| (&text[h.span.start..h.span.end], h.class))
            .collect::<Vec<_>>(),
        [
            ("(", Punctuation),
            ("locked", Keyword),
            ("when", Keyword),
            ("c", Name),
            (")", Punctuation),
        ]
    );
    // Unfinished documentation colors as a string to the end of the text.
    let text = "\"\"\"\nThe radius \"of\" a\n# face";
    assert_eq!(
        highlight(text)
            .iter()
            .map(|h| (h.span.start, h.span.end, h.class))
            .collect::<Vec<_>>(),
        [(0, text.len(), String)]
    );
}
