use rust_project_scaffold::{CancellationToken, Identity, Options, Plan, ProjectKind};
use std::{collections::BTreeMap, fs, path::Path};

fn plan(target: &Path) -> Plan {
    let mut options = Options::new(target, ProjectKind::Cli);
    options.name = "demo".into();
    options.repository = "https://example.com/demo".into();
    options.author = "Test Author".into();
    let mut plan = Plan::render(
        options,
        &Identity {
            owner: "test".into(),
            author: "Test Author".into(),
        },
    )
    .unwrap();
    plan.files = BTreeMap::from([("nested/code.rs".into(), b"generated".to_vec())]);
    plan
}

#[test]
fn publication_rejects_nonempty_targets_and_preserves_unrelated_files_with_force() {
    let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    fs::write(directory.path().join("personal.txt"), "keep").unwrap();
    let mut plan = plan(directory.path());
    assert!(plan.publish(&CancellationToken::new()).is_err());
    assert!(!directory.path().join("nested").exists());
    plan.options.force = true;
    plan.publish(&CancellationToken::new()).unwrap();
    assert_eq!(
        fs::read(directory.path().join("nested/code.rs")).unwrap(),
        b"generated"
    );
    assert_eq!(
        fs::read(directory.path().join("personal.txt")).unwrap(),
        b"keep"
    );
}

#[test]
fn invalid_plan_paths_and_cancelled_publication_leave_no_project() {
    let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    for path in ["../outside", "/outside", "", "nested/../../outside"] {
        let target = directory.path().join("project");
        let mut plan = plan(&target);
        plan.files = BTreeMap::from([(path.into(), b"escape".to_vec())]);
        assert!(plan.publish(&CancellationToken::new()).is_err());
        assert!(!target.exists());
    }
    let token = CancellationToken::new();
    token.cancel();
    assert!(
        plan(&directory.path().join("project"))
            .publish(&token)
            .is_err()
    );
    assert!(!directory.path().join("project").exists());
}

#[test]
fn conflicting_file_and_directory_plan_is_rejected_before_any_publication() {
    let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let target = directory.path().join("project");
    let mut project = plan(&target);
    project.files = BTreeMap::from([
        ("nested".into(), b"file".to_vec()),
        ("nested/code.rs".into(), b"inside".to_vec()),
    ]);
    assert!(project.publish(&CancellationToken::new()).is_err());
    assert!(
        !target.exists(),
        "invalid plan must fail before creating directories"
    );
}

#[cfg(unix)]
#[test]
fn force_preserves_file_permissions_and_extended_attributes() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    fs::create_dir(directory.path().join("nested")).unwrap();
    let file = directory.path().join("nested/code.rs");
    fs::write(&file, "old").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();
    xattr::set(&file, "user.scaffold-test", b"preserved").unwrap();
    let mut plan = plan(directory.path());
    plan.options.force = true;
    plan.publish(&CancellationToken::new()).unwrap();
    assert_eq!(fs::read(&file).unwrap(), b"generated");
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(
        xattr::get(&file, "user.scaffold-test").unwrap().unwrap(),
        b"preserved"
    );
}

#[cfg(unix)]
#[test]
fn symlink_destinations_and_ancestors_are_rejected_without_external_writes() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let outside = directory.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("code.rs"), "original").unwrap();
    let target = directory.path().join("project");
    fs::create_dir(&target).unwrap();
    symlink(&outside, target.join("nested")).unwrap();
    let mut project = plan(&target);
    project.options.force = true;
    assert!(project.publish(&CancellationToken::new()).is_err());
    assert_eq!(fs::read(outside.join("code.rs")).unwrap(), b"original");
    fs::remove_file(target.join("nested")).unwrap();
    fs::create_dir(target.join("nested")).unwrap();
    symlink(outside.join("code.rs"), target.join("nested/code.rs")).unwrap();
    assert!(project.publish(&CancellationToken::new()).is_err());
    assert_eq!(fs::read(outside.join("code.rs")).unwrap(), b"original");
    let link = directory.path().join("link");
    symlink(&target, &link).unwrap();
    let project = plan(&link.join("new"));
    assert!(project.publish(&CancellationToken::new()).is_err());
    assert!(!target.join("new").exists());
}
