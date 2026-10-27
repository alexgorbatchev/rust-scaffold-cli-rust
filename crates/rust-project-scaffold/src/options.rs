use crate::{Programs, Result, invalid};
use chrono::{Datelike, Local};
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectKind {
    Cli,
    Library,
}
impl ProjectKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Library => "lib",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub kind: ProjectKind,
    pub target: PathBuf,
    pub name: String,
    pub binary: String,
    pub package: String,
    pub repository: String,
    pub description: String,
    pub author: String,
    pub year: i32,
    pub rust_version: String,
    pub edition: String,
    pub created_date: String,
    pub force: bool,
    pub dry_run: bool,
    pub init_git: bool,
    pub run_lockfile: bool,
}
impl Options {
    pub fn new(target: impl Into<PathBuf>, kind: ProjectKind) -> Self {
        let now = Local::now();
        Self {
            kind,
            target: target.into(),
            name: String::new(),
            binary: String::new(),
            package: String::new(),
            repository: String::new(),
            description: String::new(),
            author: String::new(),
            year: now.year(),
            rust_version: "1.85".into(),
            edition: "2024".into(),
            created_date: now.format("%Y-%m-%d %H:%M").to_string(),
            force: false,
            dry_run: false,
            init_git: true,
            run_lockfile: true,
        }
    }

    pub fn normalize(mut self, identity: &Identity) -> Result<Self> {
        let target = if self.target.as_os_str().is_empty() {
            Path::new(".")
        } else {
            &self.target
        };
        self.target = absolute(target)?;
        if self.name.is_empty() {
            self.name = match self.target.file_name() {
                Some(name) => name
                    .to_str()
                    .ok_or_else(|| {
                        invalid("target name is not UTF-8; supply an explicit project name")
                    })?
                    .into(),
                None => if self.kind == ProjectKind::Cli {
                    "my-cli"
                } else {
                    "mylib"
                }
                .into(),
            };
        }
        segment(&self.name, "name")?;
        if self.kind == ProjectKind::Cli {
            if self.binary.is_empty() {
                self.binary = self
                    .name
                    .strip_suffix("-cli")
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&self.name)
                    .into();
            }
            segment(&self.binary, "binary")?;
            if ["deps", "examples", "build", "incremental"].contains(&self.binary.as_str()) {
                return Err(invalid(
                    "binary conflicts with a reserved Cargo build directory name",
                ));
            }
        }
        if self.package.is_empty() {
            self.package = clean_package_name(&self.name);
        }
        if !valid_package(&self.package) {
            return Err(invalid(
                "crate-name must be an ASCII Rust identifier other than a keyword or _",
            ));
        }
        if self.description.is_empty() {
            self.description = format!(
                "A {} for {}",
                if self.kind == ProjectKind::Cli {
                    "command-line tool"
                } else {
                    "Rust library"
                },
                self.name
            );
        }
        if self.repository.is_empty() {
            self.repository = format!("https://github.com/{}/{}", identity.owner, self.name);
        }
        let repository = url::Url::parse(&self.repository)
            .map_err(|_| invalid("repository must be an HTTP or HTTPS URL with a host"))?;
        if !matches!(repository.scheme(), "http" | "https")
            || repository.host_str().is_none()
            || !repository.username().is_empty()
            || repository.password().is_some()
            || repository.query().is_some()
            || repository.fragment().is_some()
            || self.repository.chars().any(char::is_whitespace)
            || self.repository.chars().any(char::is_control)
        {
            return Err(invalid(
                "repository must be an HTTP or HTTPS URL without credentials, query, fragment, or whitespace",
            ));
        }
        if self.author.is_empty() {
            self.author.clone_from(&identity.author);
        }
        if self.author.contains(['\n', '\r', '\0']) || self.description.contains('\0') {
            return Err(invalid(
                "author must fit one line and description must not contain NUL",
            ));
        }
        validate_rust_version(&self.rust_version)?;
        if !matches!(self.edition.as_str(), "2021" | "2024") {
            return Err(invalid("edition must be 2021 or 2024"));
        }
        if self.year <= 0 || self.created_date.contains(['\n', '\r', '\0']) {
            return Err(invalid("year and creation date are invalid"));
        }
        Ok(self)
    }
}

#[derive(Clone, Debug)]
pub struct Identity {
    pub owner: String,
    pub author: String,
}
impl Identity {
    pub async fn detect(
        programs: &Programs,
        need_owner: bool,
        need_author: bool,
        token: &crate::CancellationToken,
    ) -> Result<Self> {
        crate::check_cancel(token)?;
        let username = whoami::username()
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| std::env::var("USER").ok())
            .or_else(|| std::env::var("USERNAME").ok())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "user".into());
        let mut identity = Self {
            owner: username.clone(),
            author: username,
        };
        if need_owner {
            for (program, args) in [
                (&programs.gh, vec!["api", "user", "-q", ".login"]),
                (&programs.git, vec!["config", "github.user"]),
                (&programs.git, vec!["config", "user.username"]),
            ] {
                if let Some(value) = programs.probe(program, &args, token).await?
                    && !value.contains(char::is_whitespace)
                {
                    identity.owner = value;
                    break;
                }
            }
        }
        if need_author {
            identity.author = programs
                .probe(&programs.git, &["config", "user.name"], token)
                .await?
                .or_else(|| whoami::realname().ok().filter(|s| !s.is_empty()))
                .unwrap_or(identity.author);
        }
        Ok(identity)
    }
}
pub fn clean_package_name(name: &str) -> String {
    name.replace('-', "_")
}
fn valid_package(value: &str) -> bool {
    value.is_ascii()
        && value != "_"
        && !value.starts_with("r#")
        && syn::parse_str::<syn::Ident>(value).is_ok()
        && value != "gen"
}
pub fn validate_rust_version(value: &str) -> Result<()> {
    let components = value.split('.').count();
    let expanded = if components == 2 {
        format!("{value}.0")
    } else {
        value.to_owned()
    };
    let version = semver::Version::parse(&expanded)
        .map_err(|_| invalid("rust-version must be a numeric Rust 1.85 or newer version"))?;
    if !(2..=3).contains(&components)
        || !version.pre.is_empty()
        || !version.build.is_empty()
        || version < semver::Version::new(1, 85, 0)
    {
        return Err(invalid(
            "rust-version must be a numeric Rust 1.85 or newer version",
        ));
    }
    Ok(())
}
fn segment(value: &str, name: &str) -> Result<()> {
    if value.is_empty()
        || value.starts_with('.')
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_".contains(c))
    {
        return Err(invalid(format!(
            "{name} must be a single alphanumeric, hyphen, or underscore path segment"
        )));
    }
    Ok(())
}
pub(crate) fn absolute(path: &Path) -> Result<PathBuf> {
    let path = std::path::absolute(path)?;
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                normalized.pop();
            }
            Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized)
}
