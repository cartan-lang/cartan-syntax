//! The Tree-sitter parser generated from the grammar value, compiled and
//! run against the interpreter's reading of the same text.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use cartan_syntax::{grammar, parse_document};
use tree_sitter::{Language, Parser};

/// The files `tree-sitter generate` writes under `src/`, by path relative
/// to it.
const GENERATED: &[&str] = &[
    "parser.c",
    "node-types.json",
    "tree_sitter/parser.h",
    "tree_sitter/alloc.h",
    "tree_sitter/array.h",
];

/// The scratch tree the parser is generated and compiled in: one per
/// `grammar.json`, so an unchanged grammar is not generated again.
fn tree() -> &'static Path {
    static TREE: OnceLock<PathBuf> = OnceLock::new();
    TREE.get_or_init(|| {
        let json = grammar::print_json();
        let mut hasher = std::hash::DefaultHasher::new();
        json.hash(&mut hasher);
        grammar::print_scanner().hash(&mut hasher);
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("tree-sitter-cartan")
            .join(format!("{:016x}", hasher.finish()));
        if !root.exists() {
            // generated beside the cache and moved into it whole, so a
            // failed generation leaves nothing behind to be reused
            let fresh = root.with_extension("partial");
            if fresh.exists() {
                std::fs::remove_dir_all(&fresh).unwrap();
            }
            let src = fresh.join("src");
            std::fs::create_dir_all(&src).unwrap();
            let grammar_path = src.join("grammar.json");
            std::fs::write(&grammar_path, json).unwrap();
            std::fs::write(src.join("scanner.c"), grammar::print_scanner()).unwrap();
            let mut diagnostics = Vec::new();
            tree_sitter_generate::generate_parser_in_directory(
                &fresh,
                None::<PathBuf>,
                Some(&grammar_path),
                tree_sitter_generate::ABI_VERSION_MAX,
                None,
                None,
                true,
                tree_sitter_generate::OptLevel::default(),
                &mut diagnostics,
            )
            .unwrap_or_else(|e| panic!("tree-sitter generate: {e}"));
            assert!(diagnostics.is_empty(), "{diagnostics:?}");
            assert!(GENERATED.iter().all(|path| src.join(path).exists()));
            std::fs::rename(&fresh, &root).unwrap();
        }
        root
    })
}

/// The generated parser, compiled as a shared library and loaded.
fn language() -> &'static Language {
    static LANGUAGE: OnceLock<Language> = OnceLock::new();
    LANGUAGE.get_or_init(|| {
        let src = tree().join("src");
        let library = tree().join(format!(
            "{}tree-sitter-cartan{}",
            std::env::consts::DLL_PREFIX,
            std::env::consts::DLL_SUFFIX
        ));
        if !library.exists() {
            let target = target();
            let compiler = cc::Build::new()
                .cargo_metadata(false)
                .cargo_warnings(false)
                .target(&target)
                .host(&target)
                .opt_level(0)
                .out_dir(tree())
                .std("c11")
                .include(&src)
                .get_compiler();
            let mut command = compiler.to_command();
            command
                .args(["-shared", "-fPIC", "-o"])
                .arg(&library)
                .arg(src.join("parser.c"))
                .arg(src.join("scanner.c"));
            let status = command.status().unwrap();
            assert!(status.success(), "{command:?}");
        }
        // SAFETY: the library is the generated parser, which runs no
        // initializer, and it stays loaded for the life of the process.
        unsafe {
            let library = Box::leak(Box::new(libloading::Library::new(&library).unwrap()));
            let symbol: libloading::Symbol<unsafe extern "C" fn() -> *const ()> =
                library.get(b"tree_sitter_cartan").unwrap();
            Language::new(tree_sitter_language::LanguageFn::from_raw(*symbol))
        }
    })
}

/// The target triple this test runs on, by which `cc` configures its
/// compiler outside a build script.
fn target() -> String {
    let arch = std::env::consts::ARCH;
    match std::env::consts::OS {
        "macos" => format!("{arch}-apple-darwin"),
        "linux" => format!("{arch}-unknown-linux-gnu"),
        os => panic!("compiling the parser is set up for macOS and Linux, not {os}"),
    }
}

fn parse(text: &str) -> tree_sitter::Tree {
    let mut parser = Parser::new();
    parser.set_language(language()).unwrap();
    parser.parse(text, None).unwrap()
}

/// One-line documents, each standing for one construct, that both
/// readers must read alike.
const FIXTURES: &[&str] = &[
    "x = 1",
    "# note\nx: Real = 2.5e3",
    "f(x, y) = x + y * 2^3",
    "g(x: Real): {scale: 2} = x",
    "param n = 64",
    "state u: Real = 0 (locked when n > 3)",
    "use \"lib.cart\" :: {a, b} as l",
    "fiber Cons = (rho: Real, v: Real)",
    "fiber Shape = (Circle: Real | Square)",
    "struct P = {x: Real, y: Real | None}",
    "h(k: (Real, Real) -> Real) = k(1, 2)",
    "\"\"\"A document.\"\"\"",
    "x = if a then 1 else 2",
    "x = not a and b or c",
    "x = m ?? 0",
    "x = [1, 2, 3]",
    "x = {a: 1, b => 2}",
    "x = 0 of v",
    "x = f\"e = {e:.3}\"",
    "x = u[i, j]",
    "x = 0..n",
    "x = ..",
    "x = ..5",
    "x = 3..",
    "x = ..0..n",
    "x = 0..n periodic",
    "b = [..0..ny, 0..nx]",
    "x = a[.., 1]",
    "b = [.., 0..]",
    "y = for i in 0..n { i }",
    "y = for i in .. { i }",
    "y = for i in 3.. { i }",
    "y = for i in 0..n i * 2",
    "y = each i in 0..n { i }",
    "y = fold s := 0 for i in 0..n { s + i }",
    "y = match s { Circle(r) then r, else 0 }",
    "x = { y := 2\n y }",
    "x = { [a, b] := p\n a }",
    "step { u <- u + 1 }",
    "step { u[i] <- 1 }",
    "x = view with c",
    "x = a but 0..n: v",
    "x = f(a) : style { body }",
    "require(n > 0, \"positive\")",
    // newlines: a separator where an item may end, whitespace where a
    // line has not finished, and no separator before a continuation
    "\n\nx = 1\n\n\ny = 2\n",
    "x = 1 # one\n# two\ny = 2",
    "x = if a\n    then 1\n    else 2",
    "x = a == 1\n    and b\n    or c",
    "x = a -\n    b",
    "x = 2 * (\n    a +\n    b\n)",
    "x = [1\n -2\n 3]",
    "x = [1\n +2]",
    "x = [0..4\n ..8]",
    "x = a\nwithout = 2",
    "x = v\n    with c",
    "x = f(a,\n  b)",
    "x = {\n  a: 1\n  b: 2\n}",
    "x = 0..\ny = 1",
];

#[test]
fn the_generated_parser_reads_each_construct_as_the_interpreter_does() {
    let refused: Vec<_> = FIXTURES
        .iter()
        .filter(|text| parse_document(text).has_errors())
        .collect();
    assert!(refused.is_empty(), "{refused:?}");
    let departures: Vec<_> = FIXTURES
        .iter()
        .filter_map(|text| {
            let ours = parse_document(text).root.sexp();
            let theirs = parse(text).root_node().to_sexp();
            (ours != theirs).then(|| format!("{text:?}\n  ours:   {ours}\n  theirs: {theirs}"))
        })
        .collect();
    assert!(departures.is_empty(), "{}", departures.join("\n"));
}
