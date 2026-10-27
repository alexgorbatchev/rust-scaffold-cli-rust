use cli_command_kit::Mode;
use rust_project_scaffold::{CancellationToken, Programs};
use rust_scaffold_cli::execute;
use std::ffi::OsString;

async fn run(args: &[&str], mode: Mode) -> String {
    let mut output = Vec::new();
    execute(
        args.iter().map(OsString::from),
        mode,
        &Programs::default(),
        &CancellationToken::new(),
        &mut output,
    )
    .await
    .unwrap();
    String::from_utf8(output).unwrap()
}

#[tokio::test]
async fn rust_create_options_generate_native_cargo_projects() {
    let temporary = tempfile::tempdir_in(".tmp").unwrap();
    let target = temporary.path().join("rust-library");
    let output = run(
        &[
            "rust-scaffold",
            "lib",
            "create",
            target.to_str().unwrap(),
            "--repository",
            "https://github.com/example/rust-library",
            "--rust-version",
            "1.85",
            "--edition",
            "2024",
            "--author",
            "Local Tester",
            "--no-git",
            "--no-lockfile",
        ],
        Mode::Agent,
    )
    .await;
    assert!(target.join("Cargo.toml").is_file(), "{output}");
    assert!(target.join("src/lib.rs").is_file());
    assert!(!target.join("go.mod").exists());
}

#[tokio::test]
async fn create_then_inspect_both_project_kinds() {
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    for (kind, extra, value) in [
        ("cli", "--binary", "example"),
        ("lib", "--crate-name", "example_lib"),
    ] {
        let target = temp.path().join(format!("{kind}-project"));
        let output = run(
            &[
                "rust-scaffold",
                kind,
                "create",
                target.to_str().unwrap(),
                "--repository",
                "https://github.com/example/demo",
                "--author",
                "Local Tester",
                "--no-git",
                "--no-lockfile",
                extra,
                value,
            ],
            Mode::Agent,
        )
        .await;
        assert!(output.contains("status: ok\n"), "{output}");
        assert!(output.contains("target_dir: ~/"), "{output}");
        assert!(target.join("Cargo.toml").is_file());
        let output = run(
            &["rust-scaffold", kind, "inspect", target.to_str().unwrap()],
            Mode::Agent,
        )
        .await;
        assert!(output.contains("score: 10.0\n"), "{output}");
        assert!(output.contains("issues_count: 0\n"), "{output}");
        assert!(output.contains("repository: https://github.com/example/demo\n"));
    }
}

#[tokio::test]
async fn cli_package_can_use_a_distinct_rust_crate_identifier() {
    let temporary = tempfile::tempdir_in(".tmp").unwrap();
    let target = temporary.path().join("for");
    run(
        &[
            "rust-scaffold",
            "cli",
            "create",
            target.to_str().unwrap(),
            "--crate-name",
            "for_tool",
            "--binary",
            "for-tool",
            "--repository",
            "https://github.com/example/for",
            "--author",
            "Tester",
            "--no-git",
            "--no-lockfile",
        ],
        Mode::Agent,
    )
    .await;
    let manifest = std::fs::read_to_string(target.join("Cargo.toml")).unwrap();
    assert!(manifest.contains("name = \"for_tool\""));
    let source = std::fs::read_to_string(target.join("src/main.rs")).unwrap();
    assert!(source.contains("for_tool::"), "{source}");
}

#[tokio::test]
async fn root_groups_aliases_dry_run_and_raw_version() {
    for args in [
        vec!["rust-scaffold"],
        vec!["rust-scaffold", "cli"],
        vec!["rust-scaffold", "project"],
        vec!["rust-scaffold", "lib"],
    ] {
        let text = run(&args, Mode::Agent).await;
        assert!(
            text.starts_with("ALERT: Agents must read `AGENT=1 rust-scaffold skill`"),
            "{text}"
        );
        assert!(text.contains("create"));
    }
    assert_eq!(
        run(&["rust-scaffold", "--version"], Mode::Agent).await,
        "0.1.0\n"
    );
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let target = temp.path().join("demo-cli");
    let text = run(
        &[
            "rust-scaffold",
            "project",
            "new",
            target.to_str().unwrap(),
            "--repository",
            "https://github.com/example/demo",
            "--author",
            "Tester",
            "--dry-run",
        ],
        Mode::Agent,
    )
    .await;
    assert!(text.contains("binary: demo\n"));
    assert!(text.contains("dry_run: true\n"));
    assert!(!target.exists());
}
