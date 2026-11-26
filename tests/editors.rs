//! The editor packages under `editors/`: the Tree-sitter parser generated
//! from the grammar value, compiled and run against the interpreter's
//! reading of the same text, its queries read against it, and the
//! committed tree held to the printed one.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use cartan_syntax::editors::{self, ZedGrammar, GRAMMAR_PATH, REPOSITORY};
use cartan_syntax::{grammar, parse_document};
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};

/// Set, the drift test writes the printed files instead of comparing them.
const BLESS: &str = "CARTAN_SYNTAX_BLESS";

/// Set, the revision the committed Zed manifest fetches the grammar at.
const ZED_REV: &str = "CARTAN_SYNTAX_ZED_REV";

/// The revision a first manifest names until one is pinned.
const UNPINNED: &str = "0000000000000000000000000000000000000000";

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
        // the configuration whose version the generator writes in
        let config = format!("{GRAMMAR_PATH}/tree-sitter.json");
        let printed = editors::files(&published(UNPINNED));
        let (_, configuration) = printed.iter().find(|(p, _)| *p == config).unwrap();
        let mut hasher = std::hash::DefaultHasher::new();
        json.hash(&mut hasher);
        grammar::print_scanner().hash(&mut hasher);
        configuration.hash(&mut hasher);
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
            std::fs::write(fresh.join("tree-sitter.json"), configuration).unwrap();
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
    LANGUAGE.get_or_init(|| load_language(tree()))
}

/// Compile and load a parser from its source and scanner under `root/src`.
fn load_language(root: &Path) -> Language {
    let src = root.join("src");
    let library = root.join(format!(
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
            .out_dir(root)
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
    "param n = 64",
    "state u: Real = 0 (locked when n > 3)",
    "use \"lib.cart\" :: {a, b} as l",
    "fiber Cons = (rho: Real, v: Real)",
    "fiber Shape = (Circle: Real | Square)",
    "struct P = {x: Real, y: Real | None}",
    "protocol K = Real -> Real",
    "protocol R = (Cons, Cons) -> Cons",
    "protocol P = {gamma: Real, pressure: (Real, Real) -> Real, sound: Cons -> Real}",
    "protocol P = {\n  pressure: Cons -> Real\n  flux: (Cons, Cons) -> Flux\n}",
    "face(p: {pressure: Cons -> Real, gamma: Real | None}, l: Cons) = p::pressure(l)",
    "face(p: Physics, l: Cons) = p::pressure(l)",
    "h(k: (Real, Real) -> Real) = k(1, 2)",
    "scale(gain: Real) = fn(x: Real) { gain * x }",
    "main = apply(fn(x) { y := x + 1\n y }, 3)",
    "main = fn\n(f: Real -> Real, (a, b): Pair)\n{ f(a) }",
    "fnx(x) = x",
    "\"\"\"A document.\"\"\"",
    "x = if a { 1 } else { 2 }",
    "x = if a { 1 } else if b { 2 } else { 3 }",
    "x = if a { y := 2\n y } else { 0 }",
    "x = if a { if b { 1 } } else { 2 } + 1",
    "then = 1",
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
    "y = match s { Circle(r) { r }, Square { 1 }, else { 0 } }",
    "y = match s {\n  Circle(r) { r }\n  else { 0 }\n}",
    "x = { y := 2\n y }",
    "x = { [a, b] := p\n a }",
    "step { u <- u + 1 }",
    "x = view with c",
    "x = f(a) : style { body }",
    "show(x) : {a: 1}",
    "require(n > 0, \"positive\")",
    // newlines: a separator where an item may end, whitespace where a
    // line has not finished, and no separator before a continuation
    "\n\nx = 1\n\n\ny = 2\n",
    "x = 1 # one\n# two\ny = 2",
    "x = if a { 1 }\n    else { 2 }",
    "x = a == 1\n    and b\n    or c",
    "x = a -\n    b",
    "x = 2 * (\n    a +\n    b\n)",
    "x = [1\n -2\n 3]",
    "x = [1\n +2]",
    "x = [1\n + 2\n - 3]",
    "x =\n    + a\n    + b\n    - c * 4",
    "f(x) = \n   + u\n   - v",
    "f(x) = g(x)\n    - h(x)",
    "fiber C = (a: Real)\n+ x",
    "fiber C = (a: Real)\n- x",
    "fiber C = (a: Real)\nx = 1\n    and b",
    "x = f(a)\n    : style",
    "x = { y := 2\n -y }",
    "x = [0..4\n ..8]",
    "x = a\nwithout = 2",
    "x = v\n    with c",
    "x = f(a,\n  b)",
    "x = {\n  a: 1\n  b: 2\n}",
    "x = 0..\ny = 1",
    // the merge: the loosest operator, and a bar opening a line continues
    // the line above
    "x = a | b | c",
    "x = u | e but (x: 1.0)",
    "x = a ?? b | c + d",
    "x = 0..n | m..k",
    "x = ..0..n | y",
    "x = prim | (tracer: 0.0)",
    "x = {a: 1} | each k in ks { {k => 2.5} }",
    "x = f(a | b, c)",
    "x = [a | b, c]",
    "y = for i in a | b { i }",
    "y = each x in xs | ys { x }",
    "y = fold s := a | b for i in 0..n { s }",
    "x = a\n    | b\n    | c",
    "x = a |\n    b",
    "x = a\n| b\ny = 2",
    "x = { u := a\n   | b\n u }",
    // a paren holds a description or a value until a later token decides,
    // and the bar stands in both
    "x = (a | b)",
    "x = (a | b) | c",
    "x = (A | B)(B)",
    "x = (A: Real | B)(A: 1.0)",
    "x = (k: a | b)",
    "x = (k: a | b, l: c)",
    "x = (k: a, l: b | c)",
    "x = (\n    a\n    | b\n)",
    "x = (\n    A\n    | B\n)(B)",
    "x = (k: a\n    | b,\n    l: c)",
    "fiber Shape = (\n    Circle: Real\n    | Square\n)",
    "fiber Shape = (Circle: Real\n    | Square)",
    // a brace holds an entry or an annotated local, and an annotation's
    // absence alternative stands beside a merge
    "x = {a: m | n, b: 2}",
    "x = { c: Real|None := a | b\n c }",
    "x = { c: Map<Int> := a | b\n c }",
    "x: Real|None = a | b",
    "f(x: Real|None) = x | y",
    "x = texture(|p| color)",
    "x = texture(|p| a | b)",
    "x = texture(\n  |p|\n  color\n)",
    // a parenthesized expression applied: the argument paren is immediate
    "y = (if n > 0 { f } else { g })(x)",
    "y = (f)(x, 2.0)",
    "y = (f)()",
    "y = (if c { f } else { g })(x) + 1.0",
    "y = (f)(x)[0]",
    "y = (\n    if c { f }\n    else { g }\n)(x)",
    "y = g((if c { f } else { h })(x), 2.0)",
    "y = for i in 0..n { (if i > 2 { f } else { g })(u[i]) }",
    "y = scale(2.0)(x)",
    "y = make(a)(b)(c)",
    "y = (if c { f } else { g })(a)(b)",
    "y = lib::scale(2.0)(x) + scale(3.0)(x)",
    "y = apply(scale(2.0), x)",
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

#[test]
fn both_parsers_require_a_parameter_list_and_a_braced_function_body() {
    for text in [
        "scale = fn x { x }",
        "scale = fn(x: ) { x }",
        "scale = fn(x) x",
        "scale = fn(x) { x",
    ] {
        assert!(parse_document(text).has_errors(), "{text}");
        assert!(parse(text).root_node().has_error(), "{text}");
    }
}

/// The Zed grammar source the committed manifest names.
fn published(rev: &str) -> ZedGrammar {
    ZedGrammar {
        repository: REPOSITORY.to_string(),
        rev: rev.to_string(),
    }
}

/// The owned editor files, by path from the repository root.
fn printed(zed: &ZedGrammar) -> Vec<(String, String)> {
    let mut files = editors::files(zed);
    let src = tree().join("src");
    for path in GENERATED {
        let text = std::fs::read_to_string(src.join(path)).unwrap();
        files.push((format!("{GRAMMAR_PATH}/src/{path}"), text));
    }
    files.sort();
    files
}

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// The revision the committed manifest names, which the drift test
/// keeps: no file can name the commit that holds it.
fn committed_rev() -> Option<String> {
    let text =
        std::fs::read_to_string(repository_root().join("editors/zed/extension.toml")).ok()?;
    let line = text.lines().find(|l| l.starts_with("rev = \""))?;
    Some(
        line.trim_start_matches("rev = \"")
            .trim_end_matches('"')
            .to_string(),
    )
}

fn write(root: &Path, files: &[(String, String)]) {
    for (path, text) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

fn check_or_write_editors(root: &Path, files: &[(String, String)], bless: bool) {
    if bless {
        write(root, files);
        return;
    }
    let differing: Vec<_> = files
        .iter()
        .filter(|(path, text)| std::fs::read_to_string(root.join(path)).ok().as_ref() != Some(text))
        .map(|(path, _)| path)
        .collect();
    assert!(
        differing.is_empty(),
        "editor files differ from their generated contents: {differing:?}. \
         Regenerate with {BLESS}=1 cargo test --test editors"
    );
}

fn scratch(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    root
}

/// The Zed extension as a local development copy under `out/zed/`, which
/// fetches the grammar from this checkout at the commit `HEAD` names.
fn write_zed_development_copy() {
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(repository_root())
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let Some(head) = git(&["rev-parse", "HEAD"]) else {
        eprintln!("no Git revision to point the Zed development copy at; not written");
        return;
    };
    if git(&["status", "--porcelain", "--", GRAMMAR_PATH]).is_some_and(|s| !s.is_empty()) {
        eprintln!(
            "{GRAMMAR_PATH} has uncommitted changes, which the development copy at {head} lacks"
        );
    }
    let zed = ZedGrammar {
        repository: format!("file://{}", repository_root().display()),
        rev: head,
    };
    let copy: Vec<(String, String)> = editors::files(&zed)
        .into_iter()
        .filter_map(|(path, text)| Some((path.strip_prefix("editors/")?.to_string(), text)))
        .filter(|(path, _)| path.starts_with("zed/"))
        .collect();
    write(&repository_root().join("out"), &copy);
}

#[test]
fn the_committed_editor_files_are_the_printed_ones() {
    let rev = std::env::var(ZED_REV)
        .ok()
        .or_else(committed_rev)
        .unwrap_or_else(|| UNPINNED.to_string());
    let files = printed(&published(&rev));
    let bless = std::env::var_os(BLESS).is_some();
    check_or_write_editors(repository_root(), &files, bless);
    if bless {
        write_zed_development_copy();
    }
}

#[test]
fn regeneration_and_drift_preserve_zed_build_products() {
    let root = scratch("editor-generation");
    let manifest = root.join("editors/zed/extension.toml");
    let grammars = root.join("editors/zed/grammars");
    let unowned = [
        (
            grammars.join("cartan/.git/HEAD"),
            b"ref: refs/heads/main\n".as_slice(),
        ),
        (
            grammars.join("cartan/editors/tree-sitter-cartan/src/parser.c"),
            b"/* fetched parser */\n".as_slice(),
        ),
        (grammars.join("cartan.wasm"), b"\0asm\x01\0\0\0".as_slice()),
    ];
    for (path, bytes) in &unowned {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    std::fs::write(&manifest, "stale manifest").unwrap();
    let files = printed(&published(UNPINNED));
    check_or_write_editors(&root, &files, true);
    check_or_write_editors(&root, &files, false);
    assert_ne!(std::fs::read(&manifest).unwrap(), b"stale manifest");
    for (path, bytes) in unowned {
        assert_eq!(std::fs::read(&path).unwrap(), bytes, "{}", path.display());
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn every_query_reads_against_the_generated_language() {
    let mut problems = Vec::new();
    for (path, text) in editors::files(&published(UNPINNED)) {
        if !path.ends_with(".scm") {
            continue;
        }
        if let Err(e) = Query::new(language(), &text) {
            problems.push(format!("{path}: {e}"));
        }
        for p in grammar::grammar().check_query(&text) {
            problems.push(format!("{path}:{}:{}: {}", p.line, p.col, p.msg));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The captures the printed highlights give the `len` bytes of `text` that
/// begin where `marker` first occurs.
fn highlighted(text: &str, marker: &str, len: usize) -> Vec<String> {
    let files = editors::files(&published(UNPINNED));
    let (_, highlights) = files
        .iter()
        .find(|(path, _)| path.ends_with("/highlights.scm"))
        .expect("a highlights query");
    let query = Query::new(language(), highlights).unwrap();
    let tree = parse(text);
    let start = text.find(marker).unwrap();
    let name = &text[start..start + len];
    let mut cursor = QueryCursor::new();
    let mut captures = cursor.captures(&query, tree.root_node(), text.as_bytes());
    let mut names = Vec::new();
    while let Some((found, index)) = captures.next() {
        let capture = found.captures()[*index];
        let range = capture.node.byte_range();
        if range == (start..start + name.len()) {
            names.push(query.capture_names()[capture.index as usize].to_string());
        }
    }
    names
}

#[test]
fn the_vscode_parser_shares_the_tree_sitter_release_of_these_tests() {
    let lock = std::fs::read_to_string(repository_root().join("Cargo.lock")).unwrap();
    let locked = lock
        .split("[[package]]")
        .find(|entry| entry.contains("\nname = \"tree-sitter\"\n"))
        .and_then(|entry| {
            entry
                .lines()
                .find_map(|line| line.strip_prefix("version = "))
        })
        .expect("tree-sitter in Cargo.lock");
    assert_eq!(locked, format!("\"{}\"", editors::TREE_SITTER));
}

#[test]
fn a_constructor_pattern_names_its_fiber_as_a_type() {
    let text = "f(p: Pair) = {\n    Pair(a, b) := p\n    a\n}\n";
    assert_eq!(highlighted(text, "Pair(", 4), ["type"]);
}

#[test]
#[ignore = "requires the pinned grammar revision in local Git history"]
fn zed_queries_read_against_the_pinned_parser() {
    let rev = committed_rev().expect("missing Zed grammar revision in extension.toml");
    let root = scratch("pinned-zed-parser");
    let inputs = [
        "parser.c",
        "scanner.c",
        "tree_sitter/alloc.h",
        "tree_sitter/array.h",
        "tree_sitter/parser.h",
    ];
    for path in inputs {
        let object = format!("{rev}:{GRAMMAR_PATH}/src/{path}");
        let output = Command::new("git")
            .args(["cat-file", "blob", &object])
            .env("GIT_NO_LAZY_FETCH", "1")
            .env("GIT_ALLOW_PROTOCOL", "")
            .current_dir(repository_root())
            .output()
            .expect("run Git to read the pinned parser");
        assert!(
            output.status.success(),
            "cannot read pinned parser input {object} from local Git: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let target = root.join("src").join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, output.stdout).unwrap();
    }

    let language = load_language(&root);
    let mut checked = 0;
    let mut problems = Vec::new();
    for (path, _) in editors::files(&published(&rev))
        .into_iter()
        .filter(|(path, _)| path.starts_with("editors/zed/") && path.ends_with(".scm"))
    {
        let text = std::fs::read_to_string(repository_root().join(&path)).unwrap();
        checked += 1;
        if let Err(error) = Query::new(&language, &text) {
            problems.push(format!("{path}: {error}"));
        }
    }
    std::fs::remove_dir_all(root).unwrap();
    assert!(checked > 0, "no Zed queries checked");
    assert!(
        problems.is_empty(),
        "Zed queries rejected by grammar revision {rev}:\n{}",
        problems.join("\n")
    );
    println!("Checked {checked} Zed queries against grammar revision {rev}");
}
