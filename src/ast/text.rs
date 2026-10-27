//! String literal decoding.
//!
//! Copyright (c) 2026 Jonathan Zrake. Licensed MIT OR Apache-2.0.

/// Quoted text: `\n` and `\t` spell control characters and a backslash before
/// any other character spells that character. `None` for a trailing backslash,
/// which the grammar never produces.
pub(super) fn quoted(content: &str) -> Option<String> {
    let mut decoded = String::with_capacity(content.len());
    let mut chars = content.chars();
    while let Some(ch) = chars.next() {
        decoded.push(if ch == '\\' {
            match chars.next()? {
                'n' => '\n',
                't' => '\t',
                escaped => escaped,
            }
        } else {
            ch
        });
    }
    Some(decoded)
}

/// Triple-quoted text keeps its characters except for two removals: one newline
/// immediately after the opening delimiter, and the common leading spaces/tabs
/// of the non-blank lines.
pub(super) fn triple(content: &str) -> String {
    triple_pieces(&[content.to_owned()]).remove(0)
}

/// The removals of [`triple`] over a raw template's text runs, which have one
/// hole between each neighbouring pair. Lines continue across holes, and a hole
/// is non-blank content, so a line opening with a hole has no indentation.
pub(super) fn triple_pieces(runs: &[String]) -> Vec<String> {
    let mut runs = runs.to_vec();
    if let Some(first) = runs.first_mut() {
        if let Some(rest) = first
            .strip_prefix("\r\n")
            .or_else(|| first.strip_prefix('\n'))
        {
            *first = rest.to_owned();
        }
    }
    let indent = common_indent(&runs);
    let mut at_line_start = true;
    runs.iter()
        .map(|run| {
            let mut out = String::with_capacity(run.len());
            for (index, segment) in run.split('\n').enumerate() {
                if index > 0 {
                    out.push('\n');
                    at_line_start = true;
                }
                if at_line_start {
                    // A blank line shorter than the indentation becomes empty.
                    out.push_str(
                        segment
                            .strip_prefix(indent.as_str())
                            .unwrap_or_else(|| segment.trim_start_matches([' ', '\t'])),
                    );
                } else {
                    out.push_str(segment);
                }
                at_line_start = false;
            }
            out
        })
        .collect()
}

/// The longest run of spaces and tabs opening every non-blank line.
fn common_indent(runs: &[String]) -> String {
    let mut indent: Option<&str> = None;
    // Each line is its leading text segment and whether a hole interrupts it.
    let mut lines: Vec<(&str, bool, bool)> = Vec::new();
    for (index, run) in runs.iter().enumerate() {
        for (line, segment) in run.split('\n').enumerate() {
            let blank = segment.trim().is_empty();
            match lines.last_mut() {
                Some((_, has_hole, all_blank)) if line == 0 && index > 0 => {
                    *has_hole = true;
                    *all_blank &= blank;
                }
                _ => lines.push((segment, false, blank)),
            }
        }
    }
    for (first, has_hole, blank) in lines {
        if blank && !has_hole {
            continue;
        }
        let ws = &first[..first.len() - first.trim_start_matches([' ', '\t']).len()];
        indent = Some(match indent {
            None => ws,
            Some(previous) => {
                let common = previous
                    .bytes()
                    .zip(ws.bytes())
                    .take_while(|(a, b)| a == b)
                    .count();
                &previous[..common]
            }
        });
    }
    indent.unwrap_or("").to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_text_reads_two_control_escapes_and_drops_other_backslashes() {
        assert_eq!(quoted(r"a\nb\tc").as_deref(), Some("a\nb\tc"));
        assert_eq!(
            quoted(r#"say \"so\" \\ \x λ"#).as_deref(),
            Some(r#"say "so" \ x λ"#)
        );
        assert_eq!(quoted("trailing\\"), None);
    }

    #[test]
    fn triple_text_is_raw_after_the_opening_newline_and_common_indentation() {
        assert_eq!(triple(r#"a\nb "so""#), r#"a\nb "so""#);
        assert_eq!(triple("\nplain\n"), "plain\n");
        assert_eq!(triple("\n    one\n      two\n    "), "one\n  two\n");
        assert_eq!(triple("\n    one\n\n    two\n    "), "one\n\ntwo\n");
        assert_eq!(triple("one\ntwo"), "one\ntwo");
        assert_eq!(triple("\n  a\n  b"), "a\nb");
        assert_eq!(triple("\r\n  a"), "a");
        assert_eq!(triple(""), "");
    }

    #[test]
    fn raw_template_runs_share_lines_across_holes() {
        let runs = |texts: &[&str]| {
            triple_pieces(
                &texts
                    .iter()
                    .map(|text| text.to_string())
                    .collect::<Vec<_>>(),
            )
        };
        assert_eq!(runs(&["\n    x is ", "\n    "]), ["x is ", "\n"]);
        // A line opening with a hole contributes no indentation.
        assert_eq!(runs(&["\n  a\n", " b"]), ["  a\n", " b"]);
        // Text after a hole on an indented line is not dedented again.
        assert_eq!(runs(&["\n  a ", "  b\n  c"]), ["a ", "  b\nc"]);
        // A control character in the text is ordinary content.
        assert_eq!(runs(&["\n  \u{1}", "\n  "]), ["\u{1}", "\n"]);
    }
}
