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
}
