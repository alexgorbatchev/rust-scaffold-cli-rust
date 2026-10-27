#![cfg(unix)]

use rust_project_scaffold::{CancellationToken, Identity, Programs};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

fn script(directory: &Path, name: &str, body: &str) -> PathBuf {
    let path = directory.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[tokio::test]
async fn identity_prefers_github_and_git_author_without_extra_owner_probes() {
    let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let gh = script(
        directory.path(),
        "gh",
        "test \"$*\" = 'api user -q .login' || exit 7\nprintf 'github-owner\\n'",
    );
    let git = script(
        directory.path(),
        "git",
        "test \"$*\" = 'config user.name' || exit 8\nprintf 'Full Author\\n'",
    );
    let programs = Programs {
        gh,
        git,
        ..Programs::default()
    };
    let identity = Identity::detect(&programs, true, true, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(identity.owner, "github-owner");
    assert_eq!(identity.author, "Full Author");
}

#[tokio::test]
async fn identity_skips_invalid_empty_or_failed_probes_and_unneeded_processes() {
    let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    for response in [
        "printf 'invalid owner'",
        "printf '\\377'",
        "exit 3",
        "printf ''",
    ] {
        let gh = script(directory.path(), "gh", response);
        let git = script(
            directory.path(),
            "git",
            "case \"$*\" in 'config github.user') printf '';; 'config user.username') printf 'fallback-owner';; *) exit 4;; esac",
        );
        let programs = Programs {
            gh,
            git,
            ..Programs::default()
        };
        let identity = Identity::detect(&programs, true, true, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(identity.owner, "fallback-owner");
        assert!(!identity.author.is_empty());
    }
    let programs = Programs {
        gh: directory.path().join("missing-gh"),
        git: directory.path().join("missing-git"),
        ..Programs::default()
    };
    let identity = Identity::detect(&programs, false, false, &CancellationToken::new())
        .await
        .unwrap();
    assert!(!identity.owner.is_empty());
    assert_eq!(identity.owner, identity.author);
    let token = CancellationToken::new();
    token.cancel();
    assert!(matches!(
        Identity::detect(&programs, true, true, &token).await,
        Err(rust_project_scaffold::Error::Cancelled)
    ));
}
