use rust_project_scaffold::{Options, ProjectKind, generate, inspect};
use tokio_util::sync::CancellationToken;

fn options(path: &std::path::Path, kind: ProjectKind) -> Options {
    let mut value = Options::new(path, kind);
    value.name = "demo-cli".into();
    value.repository = "https://example.com/demo".into();
    value.author = "Test Author".into();
    value.init_git = false;
    value.run_lockfile = false;
    value
}

#[tokio::test]
async fn renders_embedded_rust_projects_and_scores_actual_files() {
    let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    for kind in [ProjectKind::Cli, ProjectKind::Library] {
        let target = dir.path().join(kind.as_str());
        let output = generate(options(&target, kind), &CancellationToken::new())
            .await
            .unwrap();
        assert!(!output.files.is_empty());
        assert_eq!(inspect(&target, kind).unwrap().score(), 10.0);
        let manifest = std::fs::read_to_string(target.join("Cargo.toml")).unwrap();
        let manifest: toml::Table = toml::from_str(&manifest).unwrap();
        assert_eq!(
            manifest["package"]["repository"].as_str(),
            Some("https://example.com/demo")
        );
        assert!(target.join("src/lib.rs").is_file());
        if kind == ProjectKind::Cli {
            assert!(target.join("src/main.rs").is_file());
        }
    }
}

#[tokio::test]
async fn dry_run_and_invalid_paths_never_write_outside_target() {
    let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let target = dir.path().join("project");
    let mut config = options(&target, ProjectKind::Cli);
    config.dry_run = true;
    assert!(
        !generate(config.clone(), &CancellationToken::new())
            .await
            .unwrap()
            .files
            .is_empty()
    );
    assert!(!target.exists());
    config.dry_run = false;
    config.binary = "../../outside".into();
    assert!(generate(config, &CancellationToken::new()).await.is_err());
    assert!(!target.exists());
}

#[cfg(unix)]
#[tokio::test]
async fn post_publication_git_inspection_failure_reports_existing_generated_files() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    symlink(".git", directory.path().join(".git")).unwrap();
    let mut configuration = options(directory.path(), ProjectKind::Library);
    configuration.force = true;
    configuration.init_git = true;
    let error = generate(configuration, &CancellationToken::new())
        .await
        .unwrap_err();
    assert!(
        matches!(error, rust_project_scaffold::Error::Published(_)),
        "{error}"
    );
    assert!(directory.path().join("Cargo.toml").is_file());
}

#[tokio::test]
async fn generated_manifest_round_trips_quoted_metadata_and_dynamic_skill_path() {
    let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let target = dir.path().join("project");
    let mut config = options(&target, ProjectKind::Cli);
    let description = "Read \"quoted\" descriptions\nwith newlines and \\slashes and \u{7f}";
    config.description = description.into();
    config.author = "Author \"quoted\" \\name".into();
    config.package = "custom_crate".into();
    config.binary = "custom-tool".into();
    generate(config, &CancellationToken::new()).await.unwrap();
    let manifest: toml::Table =
        toml::from_str(&std::fs::read_to_string(target.join("Cargo.toml")).unwrap()).unwrap();
    assert_eq!(
        manifest["package"]["description"].as_str(),
        Some(description)
    );
    assert_eq!(
        manifest["package"]["authors"][0].as_str(),
        Some("Author \"quoted\" \\name")
    );
    assert_eq!(manifest["lib"]["name"].as_str(), Some("custom_crate"));
    assert_eq!(manifest["bin"][0]["name"].as_str(), Some("custom-tool"));
    assert!(target.join("skills/custom-tool/SKILL.md").is_file());
    assert!(!target.join("skills/{{ binary }}").exists());
    for entry in walkdir::WalkDir::new(target.join("src")) {
        let entry = entry.unwrap();
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "rs")
        {
            syn::parse_file(&std::fs::read_to_string(entry.path()).unwrap()).unwrap();
        }
    }
}
