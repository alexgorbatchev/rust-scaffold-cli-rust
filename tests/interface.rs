use cli_command_kit::Mode;
use rust_project_scaffold::{CancellationToken, Programs};
use rust_scaffold_cli::{Error, SKILL, command, execute, help_options};
use std::{
    ffi::OsString,
    io::{self, Write},
};

async fn text(args: &[&str], mode: Mode) -> String {
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

fn visit(tree: &clap::Command, args: Vec<String>, paths: &mut Vec<Vec<String>>) {
    paths.push(args.clone());
    for child in tree.get_subcommands() {
        let mut path = args.clone();
        path.push(child.get_name().into());
        visit(child, path, paths);
    }
}

#[tokio::test]
async fn help_all_live_paths_alert_and_embedded_skill_work_without_checkout() {
    let mut paths = Vec::new();
    visit(
        &command(Mode::Agent),
        vec!["rust-scaffold".into()],
        &mut paths,
    );
    for path in paths {
        let mut args = path;
        if !args.iter().any(|arg| arg == "help") {
            args.push("--help".into());
        }
        for mode in [Mode::Human, Mode::Agent] {
            let mut output = Vec::new();
            execute(
                args.clone(),
                mode,
                &Programs::default(),
                &CancellationToken::new(),
                &mut output,
            )
            .await
            .unwrap();
            let text = String::from_utf8(output).unwrap();
            assert_eq!(
                text.starts_with("ALERT:"),
                mode == Mode::Agent,
                "{args:?}: {text}"
            );
        }
    }
    for mode in [Mode::Agent, Mode::Human] {
        assert_eq!(text(&["rust-scaffold", "skill"], mode).await, SKILL);
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rust-scaffold"))
        .current_dir(tempfile::tempdir_in(".tmp").unwrap().path())
        .env("AGENT", "yes")
        .arg("skill")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, SKILL.as_bytes());
    assert!(output.stderr.is_empty());
}

#[test]
fn guide_covers_the_live_interface() {
    let missing = cli_command_kit::skill_omissions(&command(Mode::Agent), &help_options(), SKILL);
    assert!(missing.is_empty(), "{}", missing.join("\n"));
}

#[tokio::test]
async fn shell_completion_uses_real_command_tree_and_omits_descriptions() {
    for shell in ["bash", "zsh", "fish", "powershell"] {
        let normal = text(&["rust-scaffold", "completion", shell], Mode::Human).await;
        let terse = text(
            &["rust-scaffold", "completion", shell, "--no-descriptions"],
            Mode::Agent,
        )
        .await;
        assert!(normal.contains("rust-scaffold"), "{shell}: {normal}");
        assert!(terse.contains("rust-scaffold"));
        assert!(terse.contains("dry-run"));
        if shell == "fish" || shell == "zsh" || shell == "powershell" {
            assert!(
                normal.contains("Skip Git initialization"),
                "{shell}: {normal}"
            );
            assert!(!terse.contains("Skip Git initialization"));
        }
    }
    let help = text(&["rust-scaffold", "completion"], Mode::Human).await;
    assert!(help.contains("powershell"));
}

#[tokio::test]
async fn parse_failures_and_output_failures_propagate_once() {
    for args in [
        vec!["rust-scaffold", "unknown"],
        vec!["rust-scaffold", "skill", "extra"],
        vec!["rust-scaffold", "lib", "create", "a", "b"],
        vec!["rust-scaffold", "lib", "create", "--binary", "bad"],
        vec![
            "rust-scaffold",
            "lib",
            "create",
            "--module",
            "example.com/old",
        ],
        vec!["rust-scaffold", "cli", "create", "--go-version", "1.26"],
        vec!["rust-scaffold", "lib", "create", "--pkg", "old"],
        vec!["rust-scaffold", "cli", "create", "--no-tidy"],
    ] {
        let error = execute(
            args,
            Mode::Agent,
            &Programs::default(),
            &CancellationToken::new(),
            &mut Vec::new(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.exit_status(), 2);
        assert!(error.diagnostic(Mode::Agent).starts_with("error:"));
    }
    struct Closed;
    impl Write for Closed {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    for args in [
        vec!["rust-scaffold"],
        vec!["rust-scaffold", "--help"],
        vec!["rust-scaffold", "--version"],
        vec!["rust-scaffold", "skill"],
        vec!["rust-scaffold", "completion", "fish"],
    ] {
        let error = execute(
            args,
            Mode::Human,
            &Programs::default(),
            &CancellationToken::new(),
            &mut Closed,
        )
        .await
        .unwrap_err();
        assert!(matches!(error, Error::Output(_)));
        assert_eq!(error.exit_status(), 1);
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rust-scaffold"))
        .env("AGENT", "1")
        .args(["cli", "inspect", ".tmp/does-not-exist"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.starts_with("ERR:"));
    assert!(!stderr.contains("Usage:"));
    assert!(output.stdout.is_empty());
}
