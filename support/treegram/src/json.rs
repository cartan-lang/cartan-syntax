//! The JSON the printers write: a small value and its text, laid out
//! one member per line with two spaces of indentation, as
//! `tree-sitter generate` and a JSON formatter lay it out.

/// A JSON value. An object keeps its members in the order given.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Str(String),
    Num(i32),
    Bool(bool),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn str(s: &str) -> Json {
        Json::Str(s.to_string())
    }

    /// The value as text, with no trailing newline.
    pub fn pretty(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out
    }

    fn write(&self, out: &mut String, indent: usize) {
        match self {
            Json::Str(s) => out.push_str(&json_str(s)),
            Json::Num(n) => out.push_str(&n.to_string()),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Arr(items) if items.is_empty() => out.push_str("[]"),
            Json::Obj(members) if members.is_empty() => out.push_str("{}"),
            Json::Arr(items) => {
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    out.push_str(&" ".repeat(indent + 2));
                    item.write(out, indent + 2);
                    out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
                }
                out.push_str(&" ".repeat(indent));
                out.push(']');
            }
            Json::Obj(members) => {
                out.push_str("{\n");
                for (i, (key, value)) in members.iter().enumerate() {
                    out.push_str(&" ".repeat(indent + 2));
                    out.push_str(&json_str(key));
                    out.push_str(": ");
                    value.write(out, indent + 2);
                    out.push_str(if i + 1 < members.len() { ",\n" } else { "\n" });
                }
                out.push_str(&" ".repeat(indent));
                out.push('}');
            }
        }
    }
}

/// A string as JSON spells it.
pub fn json_str(s: &str) -> String {
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
