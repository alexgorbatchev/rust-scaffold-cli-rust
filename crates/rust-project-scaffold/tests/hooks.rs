#![cfg(unix)]

use rust_project_scaffold::{
    CancellationToken, Error, Options, Programs, ProjectKind, generate_with,
};
use std::{fs, os::unix::fs::PermissionsExt};

fn options(target: &std::path::Path) -> Options {
    let mut options = Options::new(target, ProjectKind::Library);
    options.name = "demo".into();
    options.repository = "https://example.com/demo".into();
    options.author = "Test".into();
    options
}

#[tokio::test]
async fn cargo_hook_generates_the_lockfile_in_the_published_project() {
    let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let script = directory.path().join("cargo");
    fs::write(&script, "#!/bin/sh\n[ -f Cargo.toml ] || exit 7\n[ \"$#\" = 1 ] && [ \"$1\" = generate-lockfile ] || exit 8\nprintf 'version = 4\\n' > Cargo.lock\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let project = directory.path().join("locked");
    let mut configuration = options(&project);
    configuration.init_git = false;
    let result = generate_with(
        configuration,
        &Programs {
            cargo: script,
            ..Programs::default()
        },
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(result.lockfile_error.is_none());
    assert_eq!(
        fs::read_to_string(project.join("Cargo.lock")).unwrap(),
        "version = 4\n"
    );
}

#[tokio::test]
async fn failed_hooks_are_nonfatal_and_an_existing_git_directory_skips_initialization() {
    let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let script = directory.path().join("failure");
    fs::write(&script, "#!/bin/sh\nprintf 'hook failure' >&2\nexit 7\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let programs = Programs {
        git: script.clone(),
        cargo: script,
        ..Programs::default()
    };
    let project = directory.path().join("project");
    let result = generate_with(options(&project), &programs, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(result.warnings().count(), 2);
    assert!(
        result
            .warnings()
            .all(|warning| warning.contains("hook failure"))
    );
    assert!(project.join("Cargo.toml").is_file());
    fs::create_dir(project.join(".git")).unwrap();
    let mut configuration = options(&project);
    configuration.force = true;
    let result = generate_with(configuration, &programs, &CancellationToken::new())
        .await
        .unwrap();
    assert!(result.git_error.is_none());
    assert!(result.lockfile_error.is_some());
}

#[tokio::test]
async fn successful_git_hook_initializes_main_and_cancellation_after_publish_reports_partial_state()
{
    let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let project = directory.path().join("native-git");
    let mut configuration = options(&project);
    configuration.run_lockfile = false;
    let generated = generate_with(
        configuration,
        &Programs::default(),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(generated.git_error.is_none());
    let output = std::process::Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"main\n");

    let script = directory.path().join("long-hook");
    fs::write(&script, "#!/bin/sh\nprintf ready > hook-ready\nsleep 60\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let programs = Programs {
        cargo: script,
        ..Programs::default()
    };
    let project = directory.path().join("cancelled");
    let mut configuration = options(&project);
    configuration.init_git = false;
    let token = CancellationToken::new();
    let child_token = token.clone();
    let task =
        tokio::spawn(async move { generate_with(configuration, &programs, &child_token).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !project.join("hook-ready").exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    token.cancel();
    assert!(matches!(task.await.unwrap(), Err(Error::Published(_))));
    assert!(project.join("Cargo.toml").is_file());
}
