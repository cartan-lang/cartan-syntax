# Editor support for Cartan

The files generated under `editors/` come from the grammar value in
`src/grammar.rs`. The repository's tests use `tree-sitter-generate` to write
the parser under `tree-sitter-cartan/src/`, and treegram writes `src/scanner.c`.
Regenerate these files from the repository root.

```sh
CARTAN_SYNTAX_BLESS=1 cargo test --test editors
```

The generator checks or updates its declared files and leaves files created
by editors untouched, including Zed's downloaded checkout and compiled grammar.
Remove obsolete generated files explicitly.

The same command writes a local Zed development copy to `out/zed/`, which
Git ignores. Its grammar comes from this checkout through a `file://` URL at
the commit `HEAD` names, so commit a changed parser before writing it.

## Zed

The committed `zed/extension.toml` fetches the grammar from
https://github.com/cartan-lang/cartan-syntax at the revision its `rev` names, in
`editors/tree-sitter-cartan`. Regeneration keeps the existing `rev` by default. When
the parser changes, commit it, then generate the revision pin separately.

```sh
CARTAN_SYNTAX_ZED_REV=$(git rev-parse HEAD) CARTAN_SYNTAX_BLESS=1 cargo test --test editors
```

Run the pinned-parser check before committing the updated manifest.

```sh
cargo test --offline --test editors zed_queries_read_against_the_pinned_parser -- --ignored --exact
```

This check compiles the parser from the manifest's revision and loads the
shipped Zed queries against it. It reads only local Git objects and fails if
the revision is unavailable. The ordinary editor tests use the current grammar
and can also run from a source archive.

To use the extension locally, write the development copy as above, then in
Zed run `zed: install dev extension` and choose `out/zed`. Files ending in
`.cart` and Markdown code fences tagged `cart` are highlighted. No language
server is registered.

## VS Code

The extension colours lexical tokens with a TextMate grammar printed from
the grammar value, and colours names by their role in the syntax tree with
semantic tokens: `vscode/extension.js` runs the Tree-sitter parser, built
to WebAssembly, with the highlight query Zed uses. Package it with Node; no
account is needed.

```sh
cd editors/vscode
npm install --no-package-lock
npx @vscode/vsce package
```

Packaging first runs `vscode:prepublish`: the Tree-sitter CLI builds
`tree-sitter-cartan.wasm` from `tree-sitter-cartan/`, fetching its compiler
on first use, and `check.js` requires the expected semantic tokens from a
short fixture. The package is `cartan-0.13.0.vsix`. Install it with
**Extensions: Install from VSIX** in the Command Palette, or
`code --install-extension cartan-0.13.0.vsix`. VS Code does not update
an extension installed this way. The dependencies, the built parser and the
package are ignored by Git.

Before publishing a package, run the check inside it, so that a file the
package lacks fails rather than being found in this directory: unzip the
`.vsix` into an empty folder, copy `check.js` into its `extension/` folder,
and run `node check.js` there.

## Vim

Add `vim/` to the runtime path, for example with
`set runtimepath+=/path/to/cartan-syntax/editors/vim` in `.vimrc`.

## Sublime Text

Copy `sublime/cartan.sublime-syntax` into the `Packages/User` directory.

## Emacs

Add `emacs/` to `load-path` and `(require 'cartan-mode)`.
