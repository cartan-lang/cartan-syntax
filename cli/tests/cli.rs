use std::{fs, path::Path, process::Command};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cartan-syntax"))
}

#[test]
fn help_and_usage_have_distinct_exit_statuses_and_streams() {
    let help = cli().arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8(help.stdout.clone())
        .unwrap()
        .contains("ast <source.cart>"));
    assert!(help.stderr.is_empty());
    let no_args = cli().output().unwrap();
    assert_eq!(no_args.status.code(), Some(0));
    assert_eq!(no_args.stdout, help.stdout);
    assert!(no_args.stderr.is_empty());
    for args in [vec!["ast"], vec!["unknown"], vec!["ast", "one", "two"]] {
        let output = cli().args(args).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn file_inspection_prints_ast_items_and_spans() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/diffusion.cart");
    let output = cli().arg("ast").arg(fixture).output().unwrap();
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
    let output = cli().arg("ast").arg(&path).output().unwrap();
    fs::remove_file(&path).unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());

    let output = cli().arg("ast").arg(&path).output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}
