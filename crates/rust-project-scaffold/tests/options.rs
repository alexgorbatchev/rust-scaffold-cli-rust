use rust_project_scaffold::{Identity, Options, Plan, ProjectKind, clean_package_name};

fn identity() -> Identity {
    Identity {
        owner: "example".into(),
        author: "Example Author".into(),
    }
}

#[test]
fn defaults_derive_native_cargo_names_and_rust_identifiers() {
    let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let cli = Options::new(directory.path().join("sample-cli"), ProjectKind::Cli)
        .normalize(&identity())
        .unwrap();
    assert_eq!(cli.binary, "sample");
    assert_eq!(cli.repository, "https://github.com/example/sample-cli");
    assert_eq!(cli.package, "sample_cli");
    assert_eq!(cli.rust_version, "1.85");
    assert_eq!(cli.edition, "2024");
    assert_eq!(cli.author, "Example Author");
    assert!(cli.description.contains("sample-cli"));
    let library = Options::new(directory.path().join("sample-lib"), ProjectKind::Library)
        .normalize(&identity())
        .unwrap();
    assert_eq!(library.package, "sample_lib");
    for (name, expected) in [
        ("hello-world", "hello_world"),
        ("my_lib", "my_lib"),
        ("HELLO-world", "HELLO_world"),
        ("main", "main"),
    ] {
        assert_eq!(clean_package_name(name), expected);
    }
    let root = Options::new("/", ProjectKind::Cli)
        .normalize(&identity())
        .unwrap();
    assert_eq!(root.name, "my-cli");
    assert_eq!(
        Options::new("", ProjectKind::Library)
            .normalize(&identity())
            .unwrap()
            .target,
        std::env::current_dir().unwrap()
    );
}

#[test]
fn unsafe_names_keywords_and_malformed_configuration_fail_before_rendering() {
    for (field, value) in [
        ("name", "../escape"),
        ("binary", "a/b"),
        ("repository", "example.com/name"),
        ("repository", "file:///a/b"),
        ("repository", "https://user:secret@example.com/a"),
        ("repository", "https://example.com/a?token=secret"),
        ("repository", "https://example.com/a#fragment"),
        ("repository", " https://example.com/a"),
        ("author", "first\nsecond"),
        ("description", "bad\0description"),
        ("rust_version", "1.84.9"),
        ("rust_version", "1.85.beta"),
        ("rust_version", "1.85.0-rc.1"),
        ("rust_version", "1.85.0+build"),
        ("rust_version", "1.85.1.2"),
        ("rust_version", "1.085"),
        ("rust_version", "1.85.01"),
        ("edition", "2018"),
        ("package", "for"),
        ("package", "gen"),
        ("package", "r#type"),
        ("package", "_"),
        ("package", "1name"),
        ("package", "a-b"),
        ("binary", "deps"),
        ("binary", "examples"),
        ("binary", "build"),
        ("binary", "incremental"),
    ] {
        let mut options = Options::new(
            "project",
            if field == "package" {
                ProjectKind::Library
            } else {
                ProjectKind::Cli
            },
        );
        match field {
            "name" => options.name = value.into(),
            "binary" => options.binary = value.into(),
            "repository" => options.repository = value.into(),
            "author" => options.author = value.into(),
            "description" => options.description = value.into(),
            "rust_version" => options.rust_version = value.into(),
            "edition" => options.edition = value.into(),
            "package" => options.package = value.into(),
            _ => unreachable!(),
        }
        assert!(
            Plan::render(options, &identity()).is_err(),
            "accepted {field}: {value:?}"
        );
    }
    let mut options = Options::new("project", ProjectKind::Library);
    options.year = 0;
    assert!(options.clone().normalize(&identity()).is_err());
    options.year = 2026;
    options.created_date = "injected\nline".into();
    assert!(options.normalize(&identity()).is_err());
}

#[test]
fn crate_identifiers_follow_rust_external_crate_rules_and_allow_explicit_overrides() {
    for package in ["main", "my_crate", "Tool2", "_private"] {
        let mut options = Options::new("project", ProjectKind::Library);
        options.package = package.into();
        assert!(
            Plan::render(options, &identity()).is_ok(),
            "rejected valid Rust crate identifier {package}"
        );
    }
    for package in ["工具", "αβ", "é٢", "٢name", "a\u{301}", "a²", "a\u{200d}"] {
        let mut options = Options::new("project", ProjectKind::Library);
        options.package = package.into();
        assert!(
            Plan::render(options, &identity()).is_err(),
            "accepted invalid external crate identifier {package}"
        );
    }
    let mut options = Options::new("for", ProjectKind::Cli);
    assert!(options.clone().normalize(&identity()).is_err());
    options.package = "for_cli".into();
    options.edition = "2021".into();
    options.rust_version = "1.85.1".into();
    assert_eq!(options.normalize(&identity()).unwrap().package, "for_cli");
}

#[cfg(unix)]
#[test]
fn native_target_paths_are_preserved_and_never_mangled_into_names() {
    use std::os::unix::ffi::OsStringExt;
    let path = std::path::PathBuf::from(std::ffi::OsString::from_vec(b"project-\xff".to_vec()));
    assert!(
        Options::new(&path, ProjectKind::Cli)
            .normalize(&identity())
            .is_err()
    );
    let mut options = Options::new(&path, ProjectKind::Cli);
    options.name = "demo".into();
    let normalized = options.normalize(&identity()).unwrap();
    assert_eq!(normalized.target.file_name(), path.file_name());
}
