use rust_project_scaffold::{CancellationToken, Identity, Options, Plan, ProjectKind};
use std::{fs, path::Path, process::Command};

#[test]
fn both_generated_project_kinds_are_standalone_rust_projects() {
    for kind in [ProjectKind::Cli, ProjectKind::Library] {
        let mut options = Options::new(".tmp/generated-rust-baseline", kind);
        options.name = "demo".into();
        let plan = Plan::render(
            options,
            &Identity {
                owner: "example".into(),
                author: "Test".into(),
            },
        )
        .unwrap();
        let manifest = plan
            .files
            .get(Path::new("Cargo.toml"))
            .unwrap_or_else(|| panic!("{} generator must emit Cargo.toml", kind.as_str()));
        assert!(String::from_utf8_lossy(manifest).contains("[package]"));
        assert!(plan.files.contains_key(Path::new("src/lib.rs")));
        assert!(!plan.files.contains_key(Path::new("go.mod")));
        if kind == ProjectKind::Cli {
            assert!(plan.files.contains_key(Path::new("src/main.rs")));
        }
    }
}

/// Runs the published projects' own real subprocess, help, skill, and output tests.
/// Child workspaces never inherit instrumentation from a parent's coverage run.
#[test]
fn generated_rust_projects_pass_the_actual_offline_toolchain_and_coverage_gate() {
    let temporary = Path::new(env!("CARGO_MANIFEST_DIR")).join(".tmp");
    fs::create_dir_all(&temporary).unwrap();
    let projects = tempfile::tempdir_in(&temporary).unwrap();
    let evidence = temporary.join("generated-checks");
    fs::create_dir_all(&evidence).unwrap();
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/generated-rust-target");
    fs::create_dir_all(&target).unwrap();
    let target = target.canonicalize().unwrap();
    for kind in [ProjectKind::Cli, ProjectKind::Library] {
        let directory = projects.path().join(kind.as_str());
        let mut options = Options::new(&directory, kind);
        options.name = format!("generated-{}", kind.as_str());
        options.repository = "https://example.com/generated-project".into();
        options.author = "Test Author".into();
        options.init_git = false;
        options.run_lockfile = false;
        let plan = Plan::render(
            options,
            &Identity {
                owner: "example".into(),
                author: "Test Author".into(),
            },
        )
        .unwrap();
        plan.publish(&CancellationToken::new()).unwrap();
        let mut steps = vec![
            ("lock", vec!["generate-lockfile", "--offline"]),
            ("fmt", vec!["fmt", "--all", "--", "--check"]),
            (
                "clippy",
                vec![
                    "clippy",
                    "--locked",
                    "--offline",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ],
            ),
            (
                "test",
                vec!["test", "--locked", "--offline", "--all-targets"],
            ),
            ("doctest", vec!["test", "--locked", "--offline", "--doc"]),
            ("doc", vec!["doc", "--locked", "--offline", "--no-deps"]),
            (
                "coverage",
                vec![
                    "llvm-cov",
                    "--locked",
                    "--offline",
                    "--all-targets",
                    "--fail-under-lines",
                    "90",
                ],
            ),
        ];
        if kind == ProjectKind::Library {
            steps.push(("package", vec!["package", "--locked", "--offline"]));
        }
        for (step, arguments) in steps {
            let mut command = Command::new(env!("CARGO"));
            command
                .current_dir(&directory)
                .args(&arguments)
                .env("CARGO_TARGET_DIR", &target)
                .env("CARGO_BUILD_JOBS", "2")
                .env("CARGO_TERM_COLOR", "never");
            for (key, _) in std::env::vars_os() {
                let name = key.to_string_lossy();
                if name.starts_with("CARGO_LLVM_COV")
                    || name.starts_with("__CARGO_LLVM_COV")
                    || name.starts_with("LLVM_PROFILE_FILE")
                    || (name.starts_with("CARGO_TARGET_") && name.ends_with("_RUSTFLAGS"))
                    || matches!(
                        name.as_ref(),
                        "RUSTFLAGS"
                            | "CARGO_ENCODED_RUSTFLAGS"
                            | "RUSTDOCFLAGS"
                            | "CARGO_ENCODED_RUSTDOCFLAGS"
                            | "RUSTC_WRAPPER"
                            | "RUSTC_WORKSPACE_WRAPPER"
                    )
                {
                    command.env_remove(key);
                }
            }
            if step == "doc" {
                command.env("RUSTDOCFLAGS", "-D warnings");
            }
            let output = command.output().unwrap();
            let log = format!(
                "cargo {}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
                arguments.join(" "),
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let log_path = evidence.join(format!("{}-{step}.log", kind.as_str()));
            fs::write(&log_path, &log).unwrap();
            assert!(
                output.status.success(),
                "{} {step} failed; {}\n{log}",
                kind.as_str(),
                log_path.display()
            );
        }
    }
}
