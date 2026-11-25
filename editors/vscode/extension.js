// Cartan for VS Code. The TextMate grammar colours the lexical tokens;
// these semantic tokens colour names by their role in the syntax tree,
// read by the same Tree-sitter parser and highlight query Zed uses.

"use strict";

const fs = require("fs");
const path = require("path");
const { Parser, Language, Query } = require("web-tree-sitter");

// The query's structural captures, as a semantic token type and its
// modifiers. A parameter and a port are captured only where declared.
const TOKENS = {
  function: ["function", []],
  type: ["type", []],
  variant: ["enumMember", []],
  property: ["property", []],
  "variable.parameter": ["parameter", ["declaration"]],
  "variable.special": ["variable", ["declaration"]],
};

// The query's lexical captures, which the TextMate grammar colours.
const LEXICAL = new Set([
  "boolean",
  "comment",
  "comment.doc",
  "constant.builtin",
  "keyword",
  "number",
  "operator",
  "punctuation.bracket",
  "punctuation.delimiter",
  "punctuation.special",
  "string",
  "string.escape",
  "string.special",
]);

const TYPES = [...new Set(Object.values(TOKENS).map(([type]) => type))];
const MODIFIERS = [...new Set(Object.values(TOKENS).flatMap(([, mods]) => mods))];

// The semantic tokens of `text`, in document order: for each name the
// query captures structurally, its line, its column and length in UTF-16
// code units, its token type's index in TYPES and its modifiers as a bit
// set over MODIFIERS. Where patterns capture the same node, the earlier
// pattern wins, as in Tree-sitter's highlighter. A capture spanning lines
// is left out, since a semantic token lies on one line.
function highlight(parser, query, text) {
  const tree = parser.parse(text);
  try {
    const chosen = new Map();
    for (const capture of query.captures(tree.rootNode)) {
      const token = TOKENS[capture.name];
      const { node } = capture;
      if (!token || node.startPosition.row !== node.endPosition.row) {
        continue;
      }
      const key = `${node.startIndex}:${node.endIndex}`;
      const held = chosen.get(key);
      if (!held || capture.patternIndex < held.patternIndex) {
        chosen.set(key, { node, token, patternIndex: capture.patternIndex });
      }
    }
    return [...chosen.values()]
      .sort((a, b) => a.node.startIndex - b.node.startIndex)
      .map(({ node, token: [type, mods] }) => ({
        line: node.startPosition.row,
        character: node.startPosition.column,
        length: node.endIndex - node.startIndex,
        type: TYPES.indexOf(type),
        modifiers: mods.reduce((bits, mod) => bits | (1 << MODIFIERS.indexOf(mod)), 0),
      }));
  } finally {
    tree.delete();
  }
}

// The parser and the highlight query shipped in the extension's directory.
async function load(root) {
  await Parser.init();
  const language = await Language.load(path.join(root, "tree-sitter-cartan.wasm"));
  const source = fs.readFileSync(path.join(root, "queries", "highlights.scm"), "utf8");
  const parser = new Parser();
  parser.setLanguage(language);
  return { parser, query: new Query(language, source) };
}

async function activate(context) {
  const vscode = require("vscode");
  const { parser, query } = await load(context.extensionPath);
  const legend = new vscode.SemanticTokensLegend(TYPES, MODIFIERS);
  const provider = {
    provideDocumentSemanticTokens(document) {
      const builder = new vscode.SemanticTokensBuilder(legend);
      for (const t of highlight(parser, query, document.getText())) {
        builder.push(t.line, t.character, t.length, t.type, t.modifiers);
      }
      return builder.build();
    },
  };
  context.subscriptions.push(
    vscode.languages.registerDocumentSemanticTokensProvider({ language: "cartan" }, provider, legend),
  );
}

function deactivate() {}

module.exports = { activate, deactivate, highlight, load, TOKENS, LEXICAL, TYPES, MODIFIERS };
