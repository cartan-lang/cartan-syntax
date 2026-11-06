use cartan_diagnostics::{render, to_json, Report, Source, SourceLocation, Span};
use std::path::Path;

fn located(start: usize, end: usize) -> Report {
    Report::error(
        "test.location",
        "location",
        Some(SourceLocation {
            source: 0,
            span: Span { start, end },
        }),
    )
}

#[test]
fn unicode_tabs_eof_and_invalid_spans_never_guess_byte_columns() {
    let sources = [Source {
        name: Path::new("unicode.cart"),
        text: "é\tx\n",
    }];
    assert!(render(&sources, &located(3, 4)).contains("unicode.cart:1:5"));
    let eof = render(&sources, &located(5, 5));
    assert!(eof.contains("unicode.cart:2:1"));
    assert!(eof.contains('^'));
    for (start, end) in [(1, 2), (2, 1), (0, 99)] {
        let text = render(&sources, &located(start, end));
        assert!(text.contains(&format!(":{start}..{end}")));
        assert!(!text.contains('^'));
    }
    let unlocated = Report::error("test.unlocated", "unlocated", None);
    assert!(!render(&sources, &unlocated).contains("-->"));
    let json = to_json(&sources, &located(3, 4));
    assert_eq!(json["primary"]["span"]["start"], 3);
    assert_eq!(json["primary"]["span"]["end"], 4);
}

#[cfg(unix)]
#[test]
fn non_utf8_source_names_are_marked_lossy_in_json() {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
    let sources = [Source {
        name: Path::new(OsStr::from_bytes(b"bad\xff.cart")),
        text: "",
    }];
    let json = to_json(&sources, &Report::error("test.path", "path", None));
    assert_eq!(json["sources"][0]["name_lossy"], true);
}
