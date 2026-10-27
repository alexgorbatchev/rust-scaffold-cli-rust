use crate::{ProjectKind, Result, invalid};
use std::{
    fs,
    path::{Path, PathBuf},
};

mod syntax;

#[derive(Clone, Debug)]
pub struct Check {
    pub name: &'static str,
    pub label: &'static str,
    pub passed: bool,
    pub issue: &'static str,
}
#[derive(Clone, Debug)]
pub struct Inspection {
    pub kind: ProjectKind,
    pub target: PathBuf,
    pub package: Option<String>,
    pub repository: Option<String>,
    pub rust_version: Option<String>,
    pub edition: Option<String>,
    pub checks: Vec<Check>,
}
impl Inspection {
    pub fn score(&self) -> f64 {
        10.0 * self.checks.iter().filter(|c| c.passed).count() as f64 / self.checks.len() as f64
    }
    pub fn issues(&self) -> impl Iterator<Item = &str> {
        self.checks.iter().filter(|c| !c.passed).map(|c| c.issue)
    }
}
pub fn inspect(target: &Path, kind: ProjectKind) -> Result<Inspection> {
    let target = crate::options::absolute(target)?;
    if !fs::metadata(&target)?.is_dir() {
        return Err(invalid("target path is not a directory"));
    }
    let mut report = Inspection {
        kind,
        target: target.clone(),
        package: None,
        repository: None,
        rust_version: None,
        edition: None,
        checks: Vec::new(),
    };
    let has_manifest = regular(&target.join("Cargo.toml"))?;
    let mut clap = Vec::new();
    if has_manifest {
        let text = fs::read_to_string(target.join("Cargo.toml"))?;
        let manifest: toml::Table = toml::from_str(&text)
            .map_err(|error| invalid(format!("invalid Cargo.toml: {error}")))?;
        if let Some(package) = manifest.get("package").and_then(toml::Value::as_table) {
            report.package = string(package, "name");
            report.repository = string(package, "repository");
            report.rust_version = string(package, "rust-version");
            report.edition = string(package, "edition");
        }
        clap = dependency(&manifest, "clap");
    }
    add(
        &mut report,
        "has_cargo_manifest",
        "Cargo.toml",
        has_manifest,
        "missing Cargo.toml",
    );
    for (name, label, alternatives, issue) in [
        (
            "has_justfile",
            "justfile",
            &["justfile", "Justfile"][..],
            "missing justfile",
        ),
        (
            "has_gitignore",
            ".gitignore",
            &[".gitignore"][..],
            "missing .gitignore",
        ),
        (
            "has_ci_workflow",
            ".github/workflows/ci.yml",
            &[".github/workflows/ci.yml", ".github/workflows/ci.yaml"][..],
            "missing .github/workflows/ci.yml",
        ),
        (
            "has_license",
            "LICENSE",
            &["LICENSE"][..],
            "missing LICENSE",
        ),
        (
            "has_readme",
            "README.md",
            &["README.md"][..],
            "missing README.md",
        ),
        (
            "has_agents_doc",
            "AGENTS.md",
            &["AGENTS.md"][..],
            "missing AGENTS.md",
        ),
    ] {
        add(
            &mut report,
            name,
            label,
            any_regular(&target, alternatives)?,
            issue,
        );
    }
    let source = syntax::inspect(&target, &clap)?;
    if kind == ProjectKind::Cli {
        add(
            &mut report,
            "has_release_workflow",
            ".github/workflows/release.yml",
            any_regular(
                &target,
                &[
                    ".github/workflows/release.yml",
                    ".github/workflows/release.yaml",
                ],
            )?,
            "missing .github/workflows/release.yml",
        );
        add(
            &mut report,
            "has_agent_mode",
            "AGENT=1 dual-mode helper",
            source.agent,
            "missing AGENT=1 dual-mode helper",
        );
        add(
            &mut report,
            "has_clap",
            "Clap dependency and command",
            !clap.is_empty() && source.clap,
            "missing Clap dependency or command construction",
        );
        add(
            &mut report,
            "has_tree_help",
            "hierarchical tree help",
            source.help_template && source.tree_branch && source.tree_end,
            "missing hierarchical help template and tree branches",
        );
        add(
            &mut report,
            "has_skill",
            "embedded agent usage skill",
            source.skill_command && source.skill_file,
            "missing skill command or embedded skills/<binary>/SKILL.md",
        );
    } else {
        add(
            &mut report,
            "has_rust_source",
            "src/lib.rs",
            source.library,
            "missing valid Rust library source (src/lib.rs)",
        );
        add(
            &mut report,
            "has_rust_test",
            "Rust tests",
            source.tests,
            "missing Rust tests",
        );
    }
    Ok(report)
}
fn string(table: &toml::Table, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
}
fn dependency(manifest: &toml::Table, name: &str) -> Vec<String> {
    let mut aliases = Vec::new();
    let mut direct = |table: &toml::Table| {
        if let Some(dependencies) = table.get("dependencies").and_then(toml::Value::as_table) {
            aliases.extend(
                dependencies
                    .iter()
                    .filter(|(alias, value)| {
                        value
                            .get("package")
                            .and_then(toml::Value::as_str)
                            .unwrap_or(alias)
                            == name
                    })
                    .map(|(alias, _)| alias.replace('-', "_")),
            );
        }
    };
    direct(manifest);
    if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
        for target in targets.values().filter_map(toml::Value::as_table) {
            direct(target);
        }
    }
    aliases
}
fn add(
    report: &mut Inspection,
    name: &'static str,
    label: &'static str,
    passed: bool,
    issue: &'static str,
) {
    report.checks.push(Check {
        name,
        label,
        passed,
        issue,
    });
}
pub(super) fn regular(path: &Path) -> std::io::Result<bool> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}
fn any_regular(target: &Path, alternatives: &[&str]) -> std::io::Result<bool> {
    for relative in alternatives {
        if regular(&target.join(relative))? {
            return Ok(true);
        }
    }
    Ok(false)
}
