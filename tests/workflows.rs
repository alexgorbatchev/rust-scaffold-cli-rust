use cli_command_kit::Mode;
use rust_project_scaffold::{CancellationToken, Programs};
use rust_scaffold_cli::{Error, execute};
use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    path::Path,
};

async fn invoke(
    args: Vec<OsString>,
    mode: Mode,
    programs: &Programs,
    token: &CancellationToken,
) -> Result<String, Error> {
    let mut output = Vec::new();
    execute(args, mode, programs, token, &mut output).await?;
    Ok(String::from_utf8(output).unwrap())
}
fn arguments(kind: &str, target: &Path) -> Vec<OsString> {
    [
        OsString::from("rust-scaffold"),
        kind.into(),
        "create".into(),
        target.into(),
        "--repository".into(),
        "https://github.com/example/test".into(),
        "--author".into(),
        "Tester".into(),
        "--no-git".into(),
        "--no-lockfile".into(),
    ]
    .into()
}

#[tokio::test]
async fn human_create_all_options_force_and_inspection_issues() {
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    for (kind, extra, value) in [("cli", "-b", "tool"), ("lib", "-p", "toolbox")] {
        let target = temp.path().join(format!("{kind}-target"));
        let mut args = arguments(kind, &target);
        args.extend(
            [
                "-n",
                "custom",
                extra,
                value,
                "-d",
                "A quoted \"description\"",
                "--rust-version",
                "1.85",
                "--edition",
                "2021",
            ]
            .map(OsString::from),
        );
        let output = invoke(
            args.clone(),
            Mode::Human,
            &Programs::default(),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert!(output.contains("[OK] Created"), "{output}");
        assert!(output.contains("Next steps in "));
        assert!(output.contains(if kind == "cli" {
            "just run system status"
        } else {
            "just test"
        }));
        assert!(!output.contains("Git initialization checked"));
        let manifest = fs::read_to_string(target.join("Cargo.toml")).unwrap();
        assert!(manifest.contains("rust-version = \"1.85\""));
        assert!(manifest.contains("edition = \"2021\""));
        let error = invoke(
            args.clone(),
            Mode::Human,
            &Programs::default(),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.exit_status(), 1);
        fs::write(target.join("unrelated"), "keep").unwrap();
        args.push("--force".into());
        invoke(
            args.clone(),
            Mode::Human,
            &Programs::default(),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(
            fs::read_to_string(target.join("unrelated")).unwrap(),
            "keep"
        );
        let inspect_args = vec![
            "rust-scaffold".into(),
            kind.into(),
            "inspect".into(),
            target.clone().into_os_string(),
        ];
        let output = invoke(
            inspect_args.clone(),
            Mode::Human,
            &Programs::default(),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert!(output.contains("Standards Score: 10.0 / 10"));
        assert!(output.contains("All standard"));
        assert!(output.contains("Cargo.toml (custom)"), "{output}");
        fs::remove_file(target.join("Cargo.toml")).unwrap();
        let output = invoke(
            inspect_args,
            Mode::Human,
            &Programs::default(),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert!(output.contains("[WARN] Missing Cargo.toml"));
        assert!(output.contains("Issues found ("));
        assert!(output.contains("missing Cargo.toml"));
        // A fresh invocation proves empty Rust version selects the default.
        let mut dry = arguments(kind, &temp.path().join("dry"));
        dry.extend(["--dry-run", "--rust-version", ""].map(OsString::from));
        let output = invoke(
            dry,
            Mode::Human,
            &Programs::default(),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert!(output.starts_with("[DRY-RUN]"));
        assert!(!output.contains("Next steps"));
    }
    let output = invoke(
        vec![
            "rust-scaffold".into(),
            "cli".into(),
            "inspect".into(),
            temp.path().into(),
        ],
        Mode::Agent,
        &Programs::default(),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(output.contains("issues_count:"));
    assert!(output.contains("has_cargo_manifest: false\n"), "{output}");
    assert!(!output.contains("repository:"));
}

#[tokio::test]
async fn cancelled_operations_and_output_after_publication_are_typed() {
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let token = CancellationToken::new();
    token.cancel();
    for args in [
        arguments("cli", &temp.path().join("cancelled")),
        vec![
            "rust-scaffold".into(),
            "lib".into(),
            "inspect".into(),
            temp.path().into(),
        ],
    ] {
        let error = invoke(args, Mode::Agent, &Programs::default(), &token)
            .await
            .unwrap_err();
        assert_eq!(error.exit_status(), 130);
        assert_eq!(
            error.diagnostic(Mode::Human),
            "[ERROR] operation cancelled\n"
        );
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
    let target = temp.path().join("published");
    let error = execute(
        arguments("lib", &target),
        Mode::Agent,
        &Programs::default(),
        &CancellationToken::new(),
        &mut Closed,
    )
    .await
    .unwrap_err();
    assert!(matches!(error, Error::PublishedOutput(_)));
    assert_eq!(error.exit_status(), 6);
    assert!(target.join("Cargo.toml").exists());
    let mut args = arguments("lib", &target);
    args.push("--dry-run".into());
    let error = execute(
        args,
        Mode::Agent,
        &Programs::default(),
        &CancellationToken::new(),
        &mut Closed,
    )
    .await
    .unwrap_err();
    assert!(matches!(error, Error::Output(_)));
    assert_eq!(error.exit_status(), 1);
}

#[tokio::test]
async fn default_path_uses_child_cwd_and_raw_version_has_no_alert() {
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rust-scaffold"))
        .current_dir(temp.path())
        .env("AGENT", "true")
        .args([
            "lib",
            "init",
            "--name",
            "cwd-library",
            "--repository",
            "https://github.com/example/cwd",
            "--author",
            "Tester",
            "--no-git",
            "--no-lockfile",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(temp.path().join("src/lib.rs").is_file());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rust-scaffold"))
        .current_dir(temp.path())
        .env("AGENT", "0")
        .args(["lib", "inspect"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("Standards Score: 10.0 / 10")
    );
    for agent in ["1", "true", "yes", "0"] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_rust-scaffold"))
            .env("AGENT", agent)
            .arg("--version")
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"0.1.0\n");
        assert!(output.stderr.is_empty());
    }
}

#[cfg(unix)]
#[tokio::test]
async fn native_paths_identity_hook_warnings_and_preflight_validation() {
    use std::os::unix::{ffi::OsStringExt, fs::PermissionsExt};
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let log = temp.path().join("process-log");
    let program = temp.path().join("program");
    fs::write(&program, format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"$*\" in 'api user -q .login') printf 'owner';; 'config user.name') printf 'Full Author';; *) printf 'hook failed' >&2; exit 9;; esac\n", log.display())).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let programs = Programs {
        git: program.clone(),
        gh: program.clone(),
        cargo: program,
        ..Programs::default()
    };
    for (flag, value) in [
        ("--name", "../invalid"),
        ("--rust-version", "1.84"),
        ("--edition", "2027"),
        (
            "--repository",
            "https://user:password@example.com/repository",
        ),
        ("--crate-name", "gen"),
    ] {
        let mut output = Vec::new();
        let error = execute(
            vec!["rust-scaffold", "lib", "create", flag, value],
            Mode::Agent,
            &programs,
            &CancellationToken::new(),
            &mut output,
        )
        .await
        .unwrap_err();
        assert_eq!(error.exit_status(), 1);
        assert!(output.is_empty());
    }
    assert!(
        !log.exists(),
        "invalid inputs must not start identity processes"
    );
    for mode in [Mode::Human, Mode::Agent] {
        let target = temp.path().join(if mode == Mode::Human {
            "human"
        } else {
            "agent"
        });
        let output = invoke(
            vec![
                "rust-scaffold".into(),
                "lib".into(),
                "create".into(),
                target.into(),
            ],
            mode,
            &programs,
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        if mode == Mode::Agent {
            assert!(output.contains("warnings_count: 2\n"), "{output}");
            assert!(output.contains("repository: https://github.com/owner/agent\n"));
        } else {
            assert_eq!(output.matches("[WARN]").count(), 2);
        }
        assert!(output.contains("hook failed"));
    }
    assert!(
        fs::read_to_string(&log)
            .unwrap()
            .contains("generate-lockfile")
    );
    let target = temp
        .path()
        .join(OsString::from_vec(b"native-\xff".to_vec()));
    let mut args = arguments("lib", &target);
    args.extend(["--name", "native"].map(OsString::from));
    invoke(
        args,
        Mode::Agent,
        &Programs::default(),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(target.join("src/lib.rs").is_file());
    let output = invoke(
        vec![
            "rust-scaffold".into(),
            "lib".into(),
            "inspect".into(),
            target.into(),
        ],
        Mode::Agent,
        &Programs::default(),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(output.contains("score: 10.0"));
}

#[cfg(unix)]
#[tokio::test]
async fn successful_hooks_and_cancellation_after_publication_preserve_write_state() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let program = temp.path().join("success");
    fs::write(
        &program,
        "#!/bin/sh\ncase \"$*\" in 'init -b main'|'generate-lockfile') exit 0;; *) exit 7;; esac\n",
    )
    .unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let programs = Programs {
        git: program.clone(),
        cargo: program,
        ..Programs::default()
    };
    let mut args = arguments("cli", &temp.path().join("success-cli"));
    args.retain(|arg| arg != "--no-git" && arg != "--no-lockfile");
    let output = invoke(args, Mode::Human, &programs, &CancellationToken::new())
        .await
        .unwrap();
    assert!(output.contains("[OK] Git initialization checked\n"));
    assert!(output.contains("[OK] Generated Cargo lockfile\n"));
    assert!(!output.contains("[WARN]"));

    let ready = temp.path().join("ready");
    let git = temp.path().join("git");
    fs::write(
        &git,
        format!(
            "#!/bin/sh\nprintf '' > '{}'\nexec /bin/sleep 30\n",
            ready.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&git, fs::Permissions::from_mode(0o755)).unwrap();
    let target = temp.path().join("cancel-hook");
    let programs = Programs {
        git,
        ..Programs::default()
    };
    let mut args = arguments("lib", &target);
    args.retain(|arg| arg != "--no-git");
    let token = CancellationToken::new();
    let mut output = Vec::new();
    let future = execute(args, Mode::Agent, &programs, &token, &mut output);
    tokio::pin!(future);
    tokio::select! {
        result = &mut future => panic!("hook completed before cancellation: {result:?}"),
        result = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while !ready.exists() { tokio::task::yield_now().await; }
        }) => result.unwrap(),
    }
    token.cancel();
    let error = tokio::time::timeout(std::time::Duration::from_secs(3), future)
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(error.exit_status(), 6);
    assert!(
        error
            .diagnostic(Mode::Agent)
            .starts_with("ERR: generated files remain:")
    );
    assert!(target.join("Cargo.toml").is_file());
}

#[cfg(unix)]
#[test]
fn native_binary_ctrl_c_reaps_hook_and_reports_published_state() {
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    struct Hook {
        pid: String,
        armed: bool,
    }
    impl Drop for Hook {
        fn drop(&mut self) {
            if self.armed {
                let _ = std::process::Command::new("kill")
                    .args(["-KILL", "--", &format!("-{}", self.pid)])
                    .stderr(std::process::Stdio::null())
                    .status();
            }
        }
    }
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let temp_path = fs::canonicalize(temp.path()).unwrap();
    let ready = temp_path.join("ready");
    let git = temp_path.join("git");
    fs::write(
        &git,
        format!(
            "#!/bin/sh\nprintf '%s' $$ > '{}'\nexec /bin/sleep 30\n",
            ready.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&git, fs::Permissions::from_mode(0o755)).unwrap();
    let mut child = Child(
        std::process::Command::new(env!("CARGO_BIN_EXE_rust-scaffold"))
            .env("AGENT", "1")
            .env("PATH", &temp_path)
            .args([
                OsString::from("lib"),
                "create".into(),
                temp_path.join("project").into(),
                "--repository".into(),
                "https://github.com/example/project".into(),
                "--author".into(),
                "Tester".into(),
                "--no-lockfile".into(),
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let started = Instant::now();
    while fs::read_to_string(&ready)
        .ok()
        .is_none_or(|value| value.parse::<u32>().is_err())
    {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "hook did not become ready"
        );
        assert!(child.0.try_wait().unwrap().is_none());
        std::thread::yield_now();
    }
    let hook_pid = fs::read_to_string(&ready).unwrap();
    let mut hook = Hook {
        pid: hook_pid.clone(),
        armed: true,
    };
    let signal = std::process::Command::new("kill")
        .args(["-INT", &child.0.id().to_string()])
        .status()
        .unwrap();
    assert!(signal.success());
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "cancellation did not exit"
        );
        std::thread::yield_now();
    };
    assert_eq!(status.code(), Some(6));
    let mut stderr = String::new();
    use std::io::Read;
    child
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.contains("generated files remain: operation cancelled"));
    assert!(temp_path.join("project/Cargo.toml").is_file());
    let still_running = std::process::Command::new("kill")
        .args(["-0", hook_pid.trim()])
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(!still_running.success(), "hook was not reaped");
    hook.armed = false;
}
