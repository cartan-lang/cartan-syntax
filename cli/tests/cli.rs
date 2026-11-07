use std::{fs, path::Path, process::Command};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cartan-syntax"))
}

fn scratch(name: &str) -> std::path::PathBuf {
    let directory =
        std::env::temp_dir().join(format!("cartan-syntax-{name}-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    directory
}

fn strip_escapes(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            chars.by_ref().find(|&c| c == 'm');
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn help_and_usage_have_distinct_exit_statuses_and_streams() {
    let concise = cli().arg("-h").output().unwrap();
    assert!(concise.status.success());
    assert!(concise.stderr.is_empty());
    let concise_text = String::from_utf8(concise.stdout.clone()).unwrap();
    assert!(concise_text.contains("--ast"));
    assert!(!concise_text.contains("Command outline:"));
    let full = cli().arg("--help").output().unwrap();
    assert!(full.status.success());
    assert!(full.stderr.is_empty());
    let full_text = String::from_utf8(full.stdout).unwrap();
    assert!(full_text.contains("Command outline:\n  cartan-syntax  Check and inspect"));
    assert!(full_text.contains("[SOURCE]  Cartan source file to check"));
    let no_args = cli().output().unwrap();
    assert_eq!(no_args.status.code(), Some(0));
    assert_eq!(no_args.stdout, concise.stdout);
    assert!(no_args.stderr.is_empty());
    // Help is a flag, never a command, and the views need a source.
    for args in [
        vec!["--ast"],
        vec!["--json"],
        vec!["one", "two"],
        vec!["--unknown"],
    ] {
        let output = cli().args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn check_reports_valid_syntax_or_the_shared_rendered_diagnostic() {
    let directory = scratch("check");
    let valid = directory.join("valid.cart");
    fs::write(&valid, "param n = 32\nm = n * 2\n").unwrap();
    let output = cli().arg(&valid).output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{}: valid syntax\n", valid.display())
    );

    // The same bytes render the same report a downstream tool builds with the
    // shared adapter and renderer, for the check and the syntax tree alike.
    let invalid = directory.join("invalid λ.cart");
    let text = "s = \"naïve\"\nt = s < 1 < 2\n";
    fs::write(&invalid, text).unwrap();
    let expected: Vec<String> = cartan_syntax::ast::parse_document(text)
        .unwrap_err()
        .iter()
        .map(|error| {
            cartan_diagnostics::render(
                &[cartan_diagnostics::Source {
                    name: &invalid,
                    text,
                }],
                &cartan_syntax_diagnostics::syntax(error, 0),
            )
        })
        .collect();
    let expected = format!("{}\n", expected.join("\n"));
    assert!(expected.starts_with("error[syntax.comparison_chain]"));
    for view in [None, Some("--ast")] {
        let output = cli()
            .args(["--color", "never"])
            .arg(&invalid)
            .args(view)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert_eq!(String::from_utf8(output.stderr).unwrap(), expected);
    }
    let output = cli()
        .args(["--color", "always"])
        .arg(&invalid)
        .env_remove("NO_COLOR")
        .output()
        .unwrap();
    let colored = String::from_utf8(output.stderr).unwrap();
    assert_eq!(colored.matches('\u{1b}').count(), 2);
    assert_eq!(strip_escapes(&colored), expected);

    let missing = directory.join("missing.cart");
    let output = cli().arg(&missing).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .starts_with("error: "));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn json_results_carry_validity_diagnostics_and_the_source_ast() {
    let directory = scratch("json");
    let valid = directory.join("valid.cart");
    let text = "param n = 32\n";
    fs::write(&valid, text).unwrap();
    let invalid = directory.join("invalid.cart");
    fs::write(&invalid, "main = [1, 2, 3\n").unwrap();
    let run = |path: &Path, ast: bool| {
        let output = cli()
            .args(["--color", "always", "--json"])
            .args(ast.then_some("--ast"))
            .arg(path)
            .output()
            .unwrap();
        assert!(output.stderr.is_empty());
        assert!(!output.stdout.contains(&27), "JSON carries no escapes");
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        (output.status.code(), value)
    };

    let (status, checked) = run(&valid, false);
    assert_eq!(status, Some(0));
    assert_eq!(checked["valid"], true);
    assert_eq!(checked["diagnostics"], serde_json::json!([]));
    assert!(checked.get("items").is_none());

    let (status, tree) = run(&valid, true);
    assert_eq!(status, Some(0));
    let binding = &tree["items"][0];
    assert_eq!(binding["kind"]["Binding"]["kind"], "Parameter");
    assert_eq!(
        binding["kind"]["Binding"]["pattern"]["kind"]["Name"]["text"],
        "n"
    );
    assert_eq!(binding["span"], serde_json::json!({"start": 0, "end": 12}));

    for ast in [false, true] {
        let (status, failed) = run(&invalid, ast);
        assert_eq!(status, Some(1));
        assert_eq!(failed["valid"], false);
        let diagnostic = &failed["diagnostics"][0];
        assert_eq!(diagnostic["code"], "syntax.unclosed_delimiter");
        assert_eq!(diagnostic["related"][0]["span"]["start"], 7);
        assert!(failed.get("items").is_none());
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn the_readable_syntax_tree_shows_nesting_and_positions() {
    let directory = scratch("tree");
    let path = directory.join("source.cart");
    fs::write(&path, "f(x) = {\n  y := x * 2\n  y + 1\n}\n").unwrap();
    let view = |color: &str| {
        let output = cli()
            .args(["--color", color, "--ast"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    };
    let plain = view("never");
    for line in [
        "  Function 1:1-4:2",
        "    name: f 1:1-2",
        "    body: Body 1:8-4:2",
        "        - Local 2:3-13",
        "          value: Binary 2:8-13",
        "            operator: Multiply",
        "        - Result Binary 3:3-8",
    ] {
        assert!(plain.lines().any(|l| l == line), "{line:?} in\n{plain}");
    }
    let colored = view("always");
    assert!(colored.contains('\u{1b}'));
    assert_eq!(strip_escapes(&colored), plain);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn discovery_agrees_with_options_and_help() {
    let result = cli()
        .args(["--color", "always", "--describe-cli"])
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(!result.stdout.contains(&27));
    let schema: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(schema["schema_version"], 1);
    assert_eq!(schema["program"], "cartan-syntax");
    assert_eq!(schema["version"], env!("CARGO_PKG_VERSION"));
    let root = &schema["command"];
    assert_eq!(root["commands"], serde_json::json!([]));
    let arguments = root["arguments"].as_array().unwrap();
    let argument = |id: &str| arguments.iter().find(|arg| arg["id"] == id).unwrap();
    assert_eq!(
        argument("color")["choices"],
        serde_json::json!(["auto", "always", "never"])
    );
    assert_eq!(
        argument("color")["default_values"],
        serde_json::json!(["auto"])
    );
    let source = argument("source");
    assert_eq!(source["position"], 1);
    assert_eq!(source["required"], false);
    assert_eq!(source["arity"], serde_json::json!({"min":1,"max":1}));
    for flag in ["ast", "json"] {
        assert_eq!(argument(flag)["long"], flag);
        assert_eq!(argument(flag)["action"], "set_true");
    }
    assert_eq!(argument("help-short")["action"], "help_short");
    assert_eq!(argument("help-full")["action"], "help_long");
    let version = cli().arg("--version").output().unwrap();
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn color_respects_choice_environment_and_machine_output() {
    for args in [
        vec!["--color", "always", "--help"],
        // The color probe still reads an option written after a help flag.
        vec!["-h", "--color=always"],
        vec!["--help", "--color", "always"],
    ] {
        let result = cli().args(args).env("NO_COLOR", "1").output().unwrap();
        assert!(result.status.success());
        assert!(result.stdout.contains(&27));
    }
    for args in [
        vec!["--color=never", "--help"],
        vec!["--help"],
        vec!["--color=auto", "--help"],
    ] {
        let result = cli().args(args).env("NO_COLOR", "1").output().unwrap();
        assert!(result.status.success());
        assert!(!result.stdout.contains(&27));
    }
    let result = cli()
        .args(["--color", "always", "--unknown"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stderr.contains(&27));
    let result = cli()
        .args(["--color", "always", "missing.cart"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stderr.contains(&27));
    let result = cli()
        .args(["--color=never", "--color=always", "missing.cart"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
}

#[test]
fn sources_keep_os_paths_after_the_option_terminator() {
    let directory = scratch("paths");
    for name in ["--color=always", "source with spaces λ.cart"] {
        fs::write(directory.join(name), "param n = 32\n").unwrap();
        let result = cli()
            .current_dir(&directory)
            .args(["--color=always", "--", name])
            .output()
            .unwrap();
        assert!(result.status.success(), "{:?}", result.stderr);
        assert_eq!(
            String::from_utf8(result.stdout).unwrap(),
            format!("{name}: valid syntax\n")
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let name = std::ffi::OsString::from_vec(b"nonutf8-\xff.cart".to_vec());
        // The OS may refuse creating non-UTF-8 names (for example on macOS).
        // Parsing still accepts the OS string and reaches a typed I/O failure.
        let result = cli().current_dir(&directory).arg(name).output().unwrap();
        assert_eq!(result.status.code(), Some(1));
    }
    fs::remove_dir_all(directory).unwrap();
}
