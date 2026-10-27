use crate::{CancellationToken, Identity, Options, Programs, Result, check_cancel, invalid};
use include_dir::{Dir, include_dir};
use minijinja::{AutoEscape, Environment, UndefinedBehavior, context};
use std::{collections::BTreeMap, path::PathBuf};

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

/// An entirely rendered project. Rendering performs no filesystem writes.
#[derive(Clone, Debug)]
pub struct Plan {
    pub options: Options,
    pub files: BTreeMap<PathBuf, Vec<u8>>,
}
impl Plan {
    pub fn render(options: Options, identity: &Identity) -> Result<Self> {
        let options = options.normalize(identity)?;
        let root = TEMPLATES
            .get_dir(options.kind.as_str())
            .ok_or_else(|| invalid("missing embedded templates"))?;
        let literal = |value: &str| toml::Value::String(value.into()).to_string();
        let data = context! {
            is_cli => options.kind == crate::ProjectKind::Cli,
            name => &options.name, binary => &options.binary, crate_name => &options.package,
            repository => &options.repository, description => &options.description,
            author => &options.author, year => options.year, rust_version => &options.rust_version,
            edition => &options.edition, created_date => &options.created_date,
            name_toml => literal(&options.name), binary_toml => literal(&options.binary),
            crate_name_toml => literal(&options.package), repository_toml => literal(&options.repository),
            description_toml => literal(&options.description), author_toml => literal(&options.author),
            rust_version_toml => literal(&options.rust_version), edition_toml => literal(&options.edition),
            created_date_toml => literal(&options.created_date),
        };
        let mut environment = Environment::new();
        environment.set_undefined_behavior(UndefinedBehavior::Strict);
        environment.set_syntax(
            minijinja::syntax::SyntaxConfig::builder()
                .keep_trailing_newline(true)
                .build()?,
        );
        environment.set_auto_escape_callback(|_| AutoEscape::None);
        let mut files = BTreeMap::new();
        render_dir(root, root.path(), &environment, &data, &mut files)?;
        let common = TEMPLATES
            .get_dir("common")
            .ok_or_else(|| invalid("missing shared templates"))?;
        render_dir(common, common.path(), &environment, &data, &mut files)?;
        Ok(Self { options, files })
    }
    /// Publishes prepared files with preflight checks and rollback on failure.
    /// Existing files unrelated to the plan are retained with `force`.
    pub fn publish(&self, token: &CancellationToken) -> Result<()> {
        crate::publication::publish(self, token)
    }
}
fn render_dir(
    dir: &Dir<'_>,
    root: &std::path::Path,
    environment: &Environment<'_>,
    data: &minijinja::Value,
    files: &mut BTreeMap<PathBuf, Vec<u8>>,
) -> Result<()> {
    for file in dir.files() {
        let relative = file
            .path()
            .strip_prefix(root)
            .map_err(|e| invalid(e.to_string()))?;
        let name = relative
            .to_str()
            .ok_or_else(|| invalid("template path is not UTF-8"))?;
        let path = PathBuf::from(
            environment.render_str(name.strip_suffix(".tmpl").unwrap_or(name), data)?,
        );
        if path.as_os_str().is_empty()
            || path
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(invalid(
                "rendered template path must be relative and confined to the project",
            ));
        }
        let text = file
            .contents_utf8()
            .ok_or_else(|| invalid("template is not UTF-8"))?;
        let rendered = environment.render_named_str(name, text, data)?;
        if files.insert(path, rendered.into_bytes()).is_some() {
            return Err(invalid("templates render to the same project path"));
        }
    }
    for child in dir.dirs() {
        render_dir(child, root, environment, data, files)?;
    }
    Ok(())
}
#[derive(Clone, Debug)]
pub struct Generated {
    pub options: Options,
    pub files: Vec<PathBuf>,
    pub git_error: Option<String>,
    pub lockfile_error: Option<String>,
}
impl Generated {
    pub fn warnings(&self) -> impl Iterator<Item = &str> {
        self.git_error
            .iter()
            .chain(self.lockfile_error.iter())
            .map(String::as_str)
    }
}
pub async fn generate(options: Options, token: &CancellationToken) -> Result<Generated> {
    generate_with(options, &Programs::default(), token).await
}
pub async fn generate_with(
    options: Options,
    programs: &Programs,
    token: &CancellationToken,
) -> Result<Generated> {
    check_cancel(token)?;
    let identity = Identity::detect(
        programs,
        options.repository.is_empty(),
        options.author.is_empty(),
        token,
    )
    .await?;
    let job_token = token.child_token();
    let cancellation = job_token.clone().drop_guard();
    let plan = tokio::task::spawn_blocking(move || {
        check_cancel(&job_token)?;
        let plan = Plan::render(options, &identity)?;
        if !plan.options.dry_run {
            plan.publish(&job_token)?;
        }
        Ok::<_, crate::Error>(plan)
    })
    .await??;
    cancellation.disarm();
    let mut output = Generated {
        options: plan.options.clone(),
        files: plan.files.keys().cloned().collect(),
        git_error: None,
        lockfile_error: None,
    };
    if plan.options.dry_run {
        return Ok(output);
    }
    let initialize_git = plan.options.init_git
        && !plan
            .options
            .target
            .join(".git")
            .try_exists()
            .map_err(|error| crate::Error::Published(format!("checking Git directory: {error}")))?;
    for (enabled, program, args, destination) in [
        (
            initialize_git,
            &programs.git,
            vec!["init", "-b", "main"],
            &mut output.git_error,
        ),
        (
            plan.options.run_lockfile,
            &programs.cargo,
            vec!["generate-lockfile"],
            &mut output.lockfile_error,
        ),
    ] {
        if enabled
            && let Err(error) = programs
                .run(program, &args, Some(&plan.options.target), token)
                .await
        {
            if matches!(error, crate::Error::Cancelled) {
                return Err(crate::Error::Published(error.to_string()));
            }
            *destination = Some(error.to_string());
        }
    }
    Ok(output)
}
