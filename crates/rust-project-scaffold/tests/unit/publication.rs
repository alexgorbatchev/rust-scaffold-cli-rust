//! Filesystem regressions exercised inside the publication module.
//!
//! The public transaction is synchronous, so stepping its real commit operation
//! lets these tests reproduce OS errors without timing races or fake filesystems.
use super::*;
use crate::{Identity, Options, ProjectKind};
use std::fs;

fn fixture() -> (tempfile::TempDir, Plan) {
    let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let mut options = Options::new(directory.path(), ProjectKind::Library);
    options.name = "demo".into();
    options.repository = "https://example.com/demo".into();
    options.author = "Test Author".into();
    options.force = true;
    let mut plan = Plan::render(
        options,
        &Identity {
            owner: "test".into(),
            author: "Test Author".into(),
        },
    )
    .unwrap();
    plan.files = std::collections::BTreeMap::from([
        ("a.rs".into(), b"new a".to_vec()),
        ("b.rs".into(), b"new b".to_vec()),
    ]);
    fs::write(directory.path().join("a.rs"), "original a").unwrap();
    fs::write(directory.path().join("b.rs"), "original b").unwrap();
    (directory, plan)
}

#[test]
fn actual_commit_failure_restores_previously_replaced_original() {
    let (directory, plan) = fixture();
    let token = CancellationToken::new();
    let mut transaction = Transaction::prepare(&plan, &token).unwrap();
    transaction.commit_file(0).unwrap();
    fs::remove_file(directory.path().join("b.rs")).unwrap();
    fs::create_dir(directory.path().join("b.rs")).unwrap();
    let failure = transaction.commit_file(1).unwrap_err();
    let description = failure.to_string();
    assert_eq!(transaction.rollback(failure).to_string(), description);
    assert_eq!(
        fs::read(directory.path().join("a.rs")).unwrap(),
        b"original a"
    );
    assert!(directory.path().join("b.rs").is_dir());
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[test]
fn failed_restore_retains_recovery_original_instead_of_deleting_it() {
    let (directory, plan) = fixture();
    let token = CancellationToken::new();
    let mut transaction = Transaction::prepare(&plan, &token).unwrap();
    transaction.commit_file(0).unwrap();
    let destination = directory.path().join("a.rs");
    fs::remove_file(&destination).unwrap();
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join("concurrent.txt"), "keep").unwrap();
    let failure = transaction.rollback(crate::invalid("commit interrupted"));
    assert!(matches!(failure, crate::Error::Published(_)));
    assert_eq!(
        fs::read(destination.join("concurrent.txt")).unwrap(),
        b"keep"
    );
    let originals = walkdir::WalkDir::new(directory.path())
        .into_iter()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| fs::read(entry.path()).ok())
        .filter(|bytes| bytes == b"original a")
        .count();
    assert_eq!(
        originals, 1,
        "failed restore must leave the original recoverable"
    );
}

#[cfg(unix)]
#[test]
fn prepared_directory_handles_confine_writes_after_symlink_replacement() {
    use std::os::unix::fs::symlink;
    let (directory, mut plan) = fixture();
    let nested = directory.path().join("nested");
    let retired = directory.path().join("retired");
    let outside = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    fs::create_dir(&nested).unwrap();
    fs::write(outside.path().join("code.rs"), "external original").unwrap();
    plan.files =
        std::collections::BTreeMap::from([("nested/code.rs".into(), b"generated".to_vec())]);
    let token = CancellationToken::new();
    let mut transaction = Transaction::prepare(&plan, &token).unwrap();
    fs::rename(&nested, &retired).unwrap();
    symlink(outside.path(), &nested).unwrap();
    transaction.commit_file(0).unwrap();
    transaction.finish().unwrap();
    assert_eq!(
        fs::read(outside.path().join("code.rs")).unwrap(),
        b"external original"
    );
    assert_eq!(fs::read(retired.join("code.rs")).unwrap(), b"generated");
}

#[test]
fn cancellation_between_commits_restores_originals_and_removes_staging() {
    let (directory, plan) = fixture();
    let token = CancellationToken::new();
    let mut transaction = Transaction::prepare(&plan, &token).unwrap();
    transaction.commit_file(0).unwrap();
    token.cancel();
    assert!(matches!(
        transaction.commit_remaining(&token),
        Err(crate::Error::Cancelled)
    ));
    assert_eq!(
        fs::read(directory.path().join("a.rs")).unwrap(),
        b"original a"
    );
    assert_eq!(
        fs::read(directory.path().join("b.rs")).unwrap(),
        b"original b"
    );
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[test]
fn preparation_failure_cleans_staging_before_changing_originals() {
    let (directory, mut plan) = fixture();
    plan.files
        .insert("b.rs/inside.rs".into(), b"invalid parent".to_vec());
    // Keep b.rs as an existing file, but remove it from the generated plan so
    // preflight passes and the actual directory open fails after staging a.rs.
    plan.files.remove(std::path::Path::new("b.rs"));
    assert!(Transaction::prepare(&plan, &CancellationToken::new()).is_err());
    assert_eq!(
        fs::read(directory.path().join("a.rs")).unwrap(),
        b"original a"
    );
    assert_eq!(
        fs::read(directory.path().join("b.rs")).unwrap(),
        b"original b"
    );
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[test]
fn rollback_removes_new_files_and_created_directories_even_if_a_file_was_already_removed() {
    for remove_first in [false, true] {
        let (directory, mut plan) = fixture();
        let target = directory.path().join("new-project");
        plan.options.target = target.clone();
        plan.files = std::collections::BTreeMap::from([
            ("nested/a.rs".into(), b"new a".to_vec()),
            ("nested/b.rs".into(), b"new b".to_vec()),
        ]);
        let token = CancellationToken::new();
        let mut transaction = Transaction::prepare(&plan, &token).unwrap();
        transaction.commit_file(0).unwrap();
        if remove_first {
            fs::remove_file(target.join("nested/a.rs")).unwrap();
        }
        token.cancel();
        assert!(matches!(
            transaction.commit_remaining(&token),
            Err(crate::Error::Cancelled)
        ));
        assert!(
            !target.exists(),
            "rollback must remove its newly created project"
        );
    }
}

#[test]
fn repeating_an_individual_commit_does_not_replace_the_original_backup() {
    let (directory, plan) = fixture();
    let token = CancellationToken::new();
    let mut transaction = Transaction::prepare(&plan, &token).unwrap();
    transaction.commit_file(0).unwrap();
    transaction.commit_file(0).unwrap();
    assert_eq!(fs::read(directory.path().join("a.rs")).unwrap(), b"new a");
    let error = transaction.rollback(crate::invalid("stop"));
    assert!(matches!(error, crate::Error::Invalid(_)));
    assert_eq!(
        fs::read(directory.path().join("a.rs")).unwrap(),
        b"original a"
    );
}
