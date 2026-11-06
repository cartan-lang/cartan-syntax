//! Reserved words shared by the grammar and lexical tooling.

pub const NAMESPACE: &str = "namespace";
pub const PARAM: &str = "param";
pub const PERIODIC: &str = "periodic";
pub const LOCKED: &str = "locked";
pub const WHEN: &str = "when";

pub static KEYWORDS: [&str; 34] = [
    "use", "as", NAMESPACE, PARAM, "state", "fiber", "struct", "every", "fold", "reduce", "for",
    "each", "in", "while", "with", "but", "of", "tap", "if", "then", "else", "match", "texture",
    "and", "or", "not", "true", "false", "none", PERIODIC, LOCKED, WHEN, "module", "require",
];
