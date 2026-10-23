//! The grammar's lexical layer as a TextMate grammar: the JSON a VS
//! Code extension holds under `syntaxes/`, printed from the same
//! [`Grammar`] value `print` reads. What is read off the value, and
//! the one construct stated by name rather than derived, are in
//! [`crate::lexical`].

use crate::lexical::{self, Lexical};
use crate::pattern::Dialect;
use crate::rule::Grammar;

/// The TextMate grammar of `g`, as JSON. `scope` is the root scope —
/// `source.cartan` — whose last segment suffixes every pattern name;
/// `constants` names the word tokens that are values rather than
/// keywords (`true`, `none`), colored `constant.language`.
pub fn textmate(g: &Grammar, scope: &str, constants: &[&str]) -> String {
    let lang = scope.rsplit('.').next().unwrap_or(scope);
    let lex = Lexical::of(g, constants);
    let d = Dialect::Oniguruma;

    let mut patterns: Vec<String> = Vec::new();
    if lex.triple_string {
        patterns.push(format!(
            "    {{ \"name\": \"string.quoted.triple.{lang}\", \"begin\": \"\\\"\\\"\\\"\", \
             \"end\": \"\\\"\\\"\\\"\" }}"
        ));
    }
    for (rule, name) in [
        ("string", format!("string.quoted.double.{lang}")),
        ("comment", format!("comment.line.number-sign.{lang}")),
    ] {
        if let Some(re) = lexical::regex(g, rule, d) {
            patterns.push(pattern(&name, &re));
        }
    }
    if !lex.constants.is_empty() {
        patterns.push(pattern(
            &format!("constant.language.{lang}"),
            &format!("\\b(?:{})\\b", lex.constants.join("|")),
        ));
    }
    if let Some(re) = lexical::regex(g, "number", d) {
        patterns.push(pattern(
            &format!("constant.numeric.{lang}"),
            &format!("\\b{re}"),
        ));
    }
    if !lex.keywords.is_empty() {
        patterns.push(pattern(
            &format!("keyword.control.{lang}"),
            &format!("\\b(?:{})\\b", lex.keywords.join("|")),
        ));
    }
    if !lex.operators.is_empty() {
        patterns.push(pattern(
            &format!("keyword.operator.{lang}"),
            &lex.operator_regex(d),
        ));
    }

    let mut out = String::from("{\n");
    out.push_str(
        "  \"$schema\": \
         \"https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json\",\n",
    );
    out.push_str(&format!("  \"name\": {},\n", json_str(&g.name)));
    out.push_str(&format!("  \"scopeName\": {},\n", json_str(scope)));
    out.push_str("  \"patterns\": [\n");
    out.push_str(&patterns.join(",\n"));
    out.push_str("\n  ]\n}\n");
    out
}

fn pattern(name: &str, regex: &str) -> String {
    format!(
        "    {{ \"name\": {}, \"match\": {} }}",
        json_str(name),
        json_str(regex)
    )
}

/// A string as JSON spells it.
fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
