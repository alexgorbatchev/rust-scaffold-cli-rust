mod commands;
mod output;
use clap::{ArgMatches, Command};
use clap_complete::Generator;
use cli_command_kit::Mode;
pub use commands::{command, help_options};
use rust_project_scaffold::{CancellationToken, Identity, Options, Programs, ProjectKind};
use std::{
    ffi::OsString,
    io::{self, Write},
    path::PathBuf,
};

pub const SKILL: &str = include_str!("../skills/rust-scaffold/SKILL.md");

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Arguments(#[from] clap::Error),
    #[error(transparent)]
    Scaffold(#[from] rust_project_scaffold::Error),
    #[error(transparent)]
    Output(#[from] io::Error),
    #[error("generated files remain; output failed: {0}")]
    PublishedOutput(io::Error),
}
impl Error {
    pub fn exit_status(&self) -> i32 {
        match self {
            Self::Arguments(_) => 2,
            Self::Scaffold(rust_project_scaffold::Error::Cancelled) => 130,
            Self::Scaffold(rust_project_scaffold::Error::Published(_))
            | Self::PublishedOutput(_) => 6,
            _ => 1,
        }
    }
    pub fn diagnostic(&self, mode: Mode) -> String {
        let message = cli_command_kit::display_message(&self.to_string());
        if matches!(self, Self::Arguments(_)) {
            message
        } else {
            format!(
                "{} {message}\n",
                if mode == Mode::Agent {
                    "ERR:"
                } else {
                    "[ERROR]"
                }
            )
        }
    }
}
pub type Result<T> = std::result::Result<T, Error>;

pub async fn execute<I, T>(
    args: I,
    mode: Mode,
    programs: &Programs,
    token: &CancellationToken,
    output: &mut impl Write,
) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let mut tree = command(mode);
    let matches = match tree.try_get_matches_from_mut(args) {
        Ok(matches) => matches,
        Err(error) if !error.use_stderr() => {
            output.write_all(error.to_string().as_bytes())?;
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    if matches.get_flag("version") {
        writeln!(output, "{}", env!("CARGO_PKG_VERSION"))?;
        return Ok(());
    }
    if cli_command_kit::write_skill(&matches, &help_options(), output)? {
        return Ok(());
    }
    match matches.subcommand() {
        Some(("cli" | "lib", group)) => {
            let kind = if matches.subcommand_name() == Some("cli") {
                ProjectKind::Cli
            } else {
                ProjectKind::Library
            };
            match group.subcommand() {
                Some(("create", values)) => {
                    let options = create_options(values, kind);
                    // Validate explicit arguments before any identity subprocess can access GitHub.
                    options.clone().normalize(&Identity {
                        owner: "owner".into(),
                        author: "Author".into(),
                    })?;
                    let result =
                        rust_project_scaffold::generate_with(options, programs, token).await?;
                    output
                        .write_all(output::generated(&result, mode).as_bytes())
                        .map_err(|error| {
                            if result.options.dry_run {
                                Error::Output(error)
                            } else {
                                Error::PublishedOutput(error)
                            }
                        })?;
                }
                Some(("inspect", values)) => {
                    if token.is_cancelled() {
                        return Err(rust_project_scaffold::Error::Cancelled.into());
                    }
                    let result = rust_project_scaffold::inspect(
                        values.get_one::<PathBuf>("path").expect("path default"),
                        kind,
                    )?;
                    output.write_all(output::inspection(&result, mode).as_bytes())?;
                }
                _ => write_help(
                    tree.find_subcommand_mut(kind.as_str())
                        .expect("fixed group"),
                    output,
                )?,
            }
        }
        Some(("completion", values)) => match values.subcommand() {
            Some((name, flags)) => {
                let shell = match name {
                    "bash" => clap_complete::Shell::Bash,
                    "zsh" => clap_complete::Shell::Zsh,
                    "fish" => clap_complete::Shell::Fish,
                    "powershell" => clap_complete::Shell::PowerShell,
                    _ => unreachable!("fixed shell command tree"),
                };
                if flags.get_flag("no-descriptions") {
                    tree = commands::without_descriptions(tree);
                }
                // Some native generators' default fallible entrypoint delegates
                // to an infallible implementation that panics on writer errors.
                // Vec writes cannot fail; the external boundary remains checked.
                let mut script = Vec::new();
                shell.try_generate(&tree, &mut script)?;
                output.write_all(&script)?;
            }
            _ => write_help(
                tree.find_subcommand_mut("completion").expect("fixed group"),
                output,
            )?,
        },
        _ => write_help(&mut tree, output)?,
    }
    Ok(())
}
fn write_help(tree: &mut Command, output: &mut impl Write) -> io::Result<()> {
    output.write_all(tree.render_help().to_string().as_bytes())
}
fn create_options(values: &ArgMatches, kind: ProjectKind) -> Options {
    let mut options = Options::new(
        values.get_one::<PathBuf>("path").expect("path default"),
        kind,
    );
    for (name, value) in [
        ("name", &mut options.name),
        ("repository", &mut options.repository),
        ("description", &mut options.description),
        ("author", &mut options.author),
        ("rust-version", &mut options.rust_version),
        ("edition", &mut options.edition),
        ("crate-name", &mut options.package),
    ] {
        let argument = values.get_one::<String>(name).expect("string default");
        if !argument.is_empty() {
            value.clone_from(argument);
        }
    }
    if kind == ProjectKind::Cli {
        options
            .binary
            .clone_from(values.get_one::<String>("binary").expect("string default"));
    }
    options.force = values.get_flag("force");
    options.dry_run = values.get_flag("dry-run");
    options.init_git = !values.get_flag("no-git");
    options.run_lockfile = !values.get_flag("no-lockfile");
    options
}
