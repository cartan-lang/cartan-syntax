# Editor support for Cartan

Every file under `editors/` is generated from the grammar value in
`src/grammar.rs`; do not edit them. The parser under
`tree-sitter-cartan/src/` is written by the `tree-sitter-generate` crate in
the repository's tests, and `src/scanner.c` by treegram. A test fails when a
committed file differs from the printed one. To print them again, from the
repository root:

```sh
CARTAN_SYNTAX_BLESS=1 cargo test --test editors
```

The same command writes a local Zed development copy to `out/zed/`, which
Git ignores. Its grammar comes from this checkout through a `file://` URL at
the commit `HEAD` names, so commit a changed parser before writing it.

## Zed

The committed `zed/extension.toml` fetches the grammar from
https://github.com/cartan-lang/cartan-syntax at the revision its `rev` names, in
`editors/tree-sitter-cartan`. The test keeps the `rev` already committed, since a file
cannot name the commit that holds it. When the parser changes, commit it,
then pin the revision and commit again:

```sh
CARTAN_SYNTAX_ZED_REV=$(git rev-parse HEAD) CARTAN_SYNTAX_BLESS=1 cargo test --test editors
```

To use the extension locally, write the development copy as above, then in
Zed run `zed: install dev extension` and choose `out/zed`. Files ending in
`.cart` and Markdown code fences tagged `cart` are highlighted. No language
server is registered.

## VS Code

Copy or link `vscode/` into the extensions directory as an unpacked
extension, for example to `~/.vscode/extensions/cartan-lang.cartan-0.13.0`,
and restart VS Code.

## Vim

Add `vim/` to the runtime path, for example with
`set runtimepath+=/path/to/cartan-syntax/editors/vim` in `.vimrc`.

## Sublime Text

Copy `sublime/cartan.sublime-syntax` into the `Packages/User` directory.

## Emacs

Add `emacs/` to `load-path` and `(require 'cartan-mode)`.
