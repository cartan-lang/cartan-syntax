use std::{fs, path::Path, process::Command};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cartan-syntax"))
}

#[test]
fn help_and_usage_have_distinct_exit_statuses_and_streams() {
    let concise = cli().arg("-h").output().unwrap();
    assert!(concise.status.success());
    assert!(concise.stderr.is_empty());
    let concise_text = String::from_utf8(concise.stdout.clone()).unwrap();
    assert!(concise_text.contains("ir"));
    assert!(!concise_text.contains("Command outline:"));
    let full = cli().arg("--help").output().unwrap();
    assert!(full.status.success());
    assert!(full.stderr.is_empty());
    let full_text = String::from_utf8(full.stdout).unwrap();
    assert!(full_text.contains("Command outline:\n  cartan-syntax  Inspect Cartan source syntax"));
    assert!(full_text.contains("  cartan-syntax ir  Print the normalized source AST"));
    assert!(full_text.contains("<SOURCE>  Cartan source file"));
    let no_args = cli().output().unwrap();
    assert_eq!(no_args.status.code(), Some(0));
    assert_eq!(no_args.stdout, concise.stdout);
    assert!(no_args.stderr.is_empty());
    // Help is a flag at the selected command, never a subcommand.
    for args in [
        vec!["ir"],
        vec!["unknown"],
        vec!["ir", "one", "two"],
        vec!["help"],
        vec!["help", "ir"],
    ] {
        let output = cli().args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn file_inspection_prints_ast_items_and_spans() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/diffusion.cart");
    let output = cli().arg("ir").arg(fixture).output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Binding("));
    assert!(text.contains("While("));
    assert!(text.contains("Span {"));
}

#[test]
fn parse_and_io_failures_emit_diagnostics_without_ast_output() {
    let path = std::env::temp_dir().join(format!("cartan-syntax-cli-{}.cart", std::process::id()));
    fs::write(&path, "param n =").unwrap();
    let output = cli().arg("ir").arg(&path).output().unwrap();
    fs::remove_file(&path).unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());

    let output = cli().arg("ir").arg(&path).output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}

#[test]
fn discovery_agrees_with_options_and_nested_help() {
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
    let color = root["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|arg| arg["id"] == "color")
        .unwrap();
    assert_eq!(
        color["choices"],
        serde_json::json!(["auto", "always", "never"])
    );
    assert_eq!(color["default_values"], serde_json::json!(["auto"]));
    let ir = root["commands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|cmd| cmd["name"] == "ir")
        .unwrap();
    assert_eq!(ir["aliases"], serde_json::json!([]));
    let source = ir["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|arg| arg["id"] == "source")
        .unwrap();
    assert_eq!(source["position"], 1);
    assert_eq!(source["required"], true);
    assert_eq!(source["arity"], serde_json::json!({"min":1,"max":1}));
    assert!(root["commands"]
        .as_array()
        .unwrap()
        .iter()
        .all(|cmd| cmd["name"] != "help"));
    for command in [root, ir] {
        let arguments = command["arguments"].as_array().unwrap();
        let action = |id: &str| {
            let arg = arguments.iter().find(|arg| arg["id"] == id).unwrap();
            (
                arg["short"].clone(),
                arg["long"].clone(),
                arg["action"].clone(),
            )
        };
        assert_eq!(
            action("help-short"),
            ("h".into(), serde_json::Value::Null, "help_short".into())
        );
        assert_eq!(
            action("help-full"),
            (serde_json::Value::Null, "help".into(), "help_long".into())
        );
    }
    for (args, outline) in [(["ir", "-h"], false), (["ir", "--help"], true)] {
        let result = cli().args(args).output().unwrap();
        assert!(result.status.success());
        assert!(result.stderr.is_empty());
        let text = String::from_utf8_lossy(&result.stdout);
        assert!(text.contains("SOURCE"));
        assert_eq!(
            text.contains("Command outline:\n  cartan-syntax ir"),
            outline
        );
    }
    let version = cli().arg("--version").output().unwrap();
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn color_respects_choice_environment_and_machine_output() {
    for args in [
        vec!["--color", "always", "--help"],
        vec!["ir", "--help", "--color=always"],
        vec!["--color=always", "ir", "-h"],
        // The color probe still reads an option written after a help flag.
        vec!["ir", "-h", "--color=always"],
        vec!["ir", "--help", "--color", "always"],
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
        .args(["--color", "always", "unknown"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stderr.contains(&27));
    let result = cli()
        .args(["--color", "always", "ir", "missing.cart"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stderr.contains(&27));
    let result = cli()
        .args(["--color=never", "--color=always", "ir", "missing.cart"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
}

#[test]
fn inspection_preserves_os_paths_termination_and_plain_output() {
    let directory =
        std::env::temp_dir().join(format!("cartan-syntax-paths-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let source = "param n = 32\n";
    for name in ["--color=always", "source with spaces λ.cart"] {
        fs::write(directory.join(name), source).unwrap();
        let result = cli()
            .current_dir(&directory)
            .args(["--color=always", "ir", "--", name])
            .output()
            .unwrap();
        assert!(result.status.success(), "{:?}", result.stderr);
        assert!(!result.stdout.contains(&27));
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let name = std::ffi::OsString::from_vec(b"nonutf8-\xff.cart".to_vec());
        // The OS may refuse creating non-UTF-8 names (for example on macOS).
        // Parsing still accepts the OS string and reaches a typed I/O failure.
        let result = cli()
            .current_dir(&directory)
            .arg("ir")
            .arg(name)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn retired_ast_command_is_a_usage_error() {
    for args in [
        vec!["ast"],
        vec!["ast", "--help"],
        vec!["ast", "source.cart"],
    ] {
        let result = cli().args(args).output().unwrap();
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert!(!result.stderr.is_empty());
    }
}
