// The packaging check, run before `vsce package`: the parser built for
// the package and the shipped highlight query give the expected semantic
// tokens, and every capture the query declares is either a token or
// left to the TextMate grammar.

"use strict";

const { highlight, load, TOKENS, LEXICAL, TYPES, MODIFIERS } = require("./extension.js");

const TEXT = [
  "param n = 4",
  "fiber Pair = (a: Real, b: Real)",
  "swap(p: Pair) = {",
  "    Pair(a, b) := p",
  "    Pair(b, a)",
  "}",
  'label = "π😀" + swap(Pair(1.0, 2.0))',
  "",
].join("\n");

// Line, UTF-16 column, length, token type and modifiers. The emoji is two
// UTF-16 code units, so `swap` on the last line starts at column 16.
const EXPECTED = [
  [0, 6, 1, "variable", ["declaration"]],
  [1, 6, 4, "type", []],
  [1, 14, 1, "property", []],
  [1, 17, 4, "type", []],
  [1, 23, 1, "property", []],
  [1, 26, 4, "type", []],
  [2, 0, 4, "function", []],
  [2, 5, 1, "parameter", ["declaration"]],
  [2, 8, 4, "type", []],
  [3, 4, 4, "type", []],
  [4, 4, 4, "function", []],
  [6, 16, 4, "function", []],
  [6, 21, 4, "function", []],
];

function fail(message) {
  console.error(`check failed: ${message}`);
  process.exit(1);
}

(async () => {
  const { parser, query } = await load(__dirname);
  const unknown = query.captureNames.filter((name) => !(name in TOKENS) && !LEXICAL.has(name));
  if (unknown.length) {
    fail(`captures neither mapped nor lexical: ${unknown.join(", ")}`);
  }
  const tree = parser.parse(TEXT);
  const errors = tree.rootNode.hasError;
  tree.delete();
  if (errors) {
    fail("the fixture does not parse");
  }
  const got = highlight(parser, query, TEXT).map((t) => [
    t.line,
    t.character,
    t.length,
    TYPES[t.type],
    MODIFIERS.filter((_, i) => t.modifiers & (1 << i)),
  ]);
  if (JSON.stringify(got) !== JSON.stringify(EXPECTED)) {
    fail(`tokens\n  got      ${JSON.stringify(got)}\n  expected ${JSON.stringify(EXPECTED)}`);
  }
  console.log(`check passed: ${got.length} semantic tokens`);
})().catch((error) => fail(error.stack || String(error)));
