use rust_project_scaffold::{ProjectKind, inspect};
use std::fs;

fn directory() -> tempfile::TempDir {
    tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap()
}
fn passed(root: &std::path::Path, name: &str) -> bool {
    inspect(root, ProjectKind::Cli)
        .unwrap()
        .checks
        .iter()
        .find(|check| check.name == name)
        .unwrap()
        .passed
}
#[test]
fn inspection_reads_real_cargo_dependencies_and_rust_calls_instead_of_comment_lookalikes() {
    let directory = directory();
    let root = directory.path();
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("Cargo.toml"), "[package]\nname='test'\nversion='0.1.0'\nrust-version='1.85'\nedition='2024'\nrepository='https://example.com/test'\n# clap='4'\n").unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "// std::env::var(\"AGENT\")\nconst LOOKALIKE: &str = r#\"std::env::var(\"AGENT\")\"#;\n",
    )
    .unwrap();
    let report = inspect(root, ProjectKind::Cli).unwrap();
    assert_eq!(report.package.as_deref(), Some("test"));
    assert_eq!(
        report.repository.as_deref(),
        Some("https://example.com/test")
    );
    assert_eq!(report.rust_version.as_deref(), Some("1.85"));
    assert_eq!(report.edition.as_deref(), Some("2024"));
    assert!(!passed(root, "has_agent_mode"));
    assert!(!passed(root, "has_clap"));
    for source in [
        "fn mode() { let _ = std::env::var(\"AGENT\"); }",
        "use std::env as environment; fn mode() { let _ = environment::var_os(r#\"AGENT\"#); }",
        "use std::env::{self, var as get}; fn mode() { let _ = get(\"AGENT\"); }",
        "use std::env::{self}; fn mode() { let _ = env::var(\"AGENT\"); }",
        "mod app { use std::env::var; fn mode() { let _ = var(\"AGENT\"); } }",
        "fn mode() { use std::env::var; let _ = var(\"AGENT\"); }",
    ] {
        fs::write(root.join("src/lib.rs"), source).unwrap();
        assert!(passed(root, "has_agent_mode"), "{source}");
    }
    fs::write(root.join("Cargo.toml"), "[package]\nname='test'\n[target.'cfg(unix)'.dependencies]\ncli={package='clap',version='4'}\n").unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "use cli::Command as Cli; fn command() { let _ = Cli::new(\"skill\"); }",
    )
    .unwrap();
    assert!(passed(root, "has_clap"));
}
#[test]
fn inspection_requires_regular_files_and_reports_missing_library_assets() {
    let directory = directory();
    fs::create_dir(directory.path().join("README.md")).unwrap();
    let report = inspect(directory.path(), ProjectKind::Library).unwrap();
    assert_eq!(report.score(), 0.0);
    assert_eq!(report.issues().count(), 9);
    fs::create_dir(directory.path().join("src")).unwrap();
    fs::write(directory.path().join("src/lib.rs"), "pub fn add(a: u32, b: u32) -> u32 { a + b }\n#[cfg(test)] mod tests { #[test] fn addition() { assert_eq!(super::add(1,2),3); } }").unwrap();
    fs::write(directory.path().join("Justfile"), "test:\n    cargo test\n").unwrap();
    let report = inspect(directory.path(), ProjectKind::Library).unwrap();
    assert_eq!(report.issues().count(), 6);
    assert!((report.score() - 10.0 / 3.0).abs() < 0.001);
    assert!(inspect(&directory.path().join("src/lib.rs"), ProjectKind::Library).is_err());
    assert!(inspect(&directory.path().join("missing"), ProjectKind::Cli).is_err());
    fs::write(directory.path().join("Cargo.toml"), "[package\nnot toml").unwrap();
    assert!(inspect(directory.path(), ProjectKind::Library).is_err());
}
#[test]
fn test_only_invalid_rust_and_unrelated_lookups_do_not_count_as_agent_support() {
    let directory = directory();
    let source = directory.path().join("src");
    fs::create_dir(&source).unwrap();
    fs::create_dir(directory.path().join("tests")).unwrap();
    fs::write(
        directory.path().join("tests/agent.rs"),
        "#[test] fn mode() { let _ = std::env::var(\"AGENT\"); }",
    )
    .unwrap();
    fs::write(source.join("README.md"), "std::env::var(\"AGENT\")").unwrap();
    fs::create_dir(source.join("directory.rs")).unwrap();
    for text in [
        "this is not valid Rust; std::env::var(\"AGENT\")",
        "fn mode() { let _ = std::env::var(\"OTHER\"); }",
        "fn var(key: &str) {} fn mode() { var(\"AGENT\"); }",
        "fn mode() { std::env::var(); }",
        "#[cfg(test)] mod tests { fn mode() { std::env::var(\"AGENT\"); } }",
        "#[test] fn mode() { let _ = std::env::var(\"AGENT\"); }",
        "#[cfg(test)] mod tests { use std::env::var; } fn mode() { var(\"AGENT\"); }",
        "mod unrelated { use std::env::var; } fn mode() { var(\"AGENT\"); }",
        "fn unrelated() { use std::env::var; } fn mode() { var(\"AGENT\"); }",
        "use std::env::var; mod app { fn mode() { var(\"AGENT\"); } }",
    ] {
        fs::write(source.join("lib.rs"), text).unwrap();
        assert!(!passed(directory.path(), "has_agent_mode"), "{text}");
    }
}
#[test]
fn tree_help_and_skill_checks_require_real_ast_wiring_and_existing_embedded_file() {
    let directory = directory();
    let root = directory.path();
    fs::create_dir(root.join("src")).unwrap();
    fs::create_dir_all(root.join("skills/demo")).unwrap();
    fs::write(root.join("skills/demo/SKILL.md"), "Usage").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='demo'\n[dependencies]\nclap='4'\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "use clap::Command; const SKILL: &str = include_str!(\"../skills/demo/SKILL.md\"); fn command() { let _ = Command::new(\"demo\").subcommand(Command::new(\"skill\")).help_template(format!(\"{} {}\", \"├─\", \"╰─\")); }").unwrap();
    assert!(passed(root, "has_clap"));
    assert!(passed(root, "has_tree_help"));
    assert!(passed(root, "has_skill"));
    fs::remove_file(root.join("skills/demo/SKILL.md")).unwrap();
    assert!(!passed(root, "has_skill"));
    fs::write(
        root.join("src/lib.rs"),
        "// help_template(\"├─ ╰─\")\nconst TEXT: &str = \"Command::new(skill)\";",
    )
    .unwrap();
    assert!(!passed(root, "has_tree_help"));
    assert!(!passed(root, "has_skill"));
    assert!(!passed(root, "has_clap"));
    fs::write(
        root.join("src/lib.rs"),
        "use clap::Parser; #[derive(Parser)] struct Cli {}\n",
    )
    .unwrap();
    assert!(passed(root, "has_clap"));
}
