use clap::{Arg, ArgAction, Command, value_parser};
use cli_command_kit::{HelpOptions, Mode};
use std::path::PathBuf;

pub fn help_options() -> HelpOptions {
    HelpOptions {
        skill: Some(crate::SKILL.into()),
        ..HelpOptions::default()
    }
}

fn path() -> Arg {
    Arg::new("path")
        .value_parser(value_parser!(PathBuf))
        .default_value(".")
        .help("Project directory")
}

fn create(library: bool) -> Command {
    let mut command = Command::new("create")
        .visible_aliases(["init", "new"])
        .about("Create a Rust project with tests, documentation, and automation")
        .arg(path());
    for (name, short, help) in [
        (
            "name",
            Some('n'),
            "Project name; defaults to the directory name",
        ),
        (
            "repository",
            Some('r'),
            "Repository URL; defaults to https://github.com/<owner>/<name>",
        ),
        ("description", Some('d'), "Short project description"),
        (
            "author",
            Some('a'),
            "Author; defaults to Git configuration or system user",
        ),
        (
            "rust-version",
            None,
            "Minimum Rust version; defaults to 1.85",
        ),
        (
            "edition",
            None,
            "Rust edition: 2021 or 2024; defaults to 2024",
        ),
        (
            "crate-name",
            Some('p'),
            "Rust crate identifier; defaults to the project name with underscores",
        ),
    ] {
        let mut arg = Arg::new(name).long(name).default_value("").help(help);
        if let Some(short) = short {
            arg = arg.short(short);
        }
        command = command.arg(arg);
    }
    if !library {
        command = command.arg(
            Arg::new("binary")
                .long("binary")
                .short('b')
                .default_value("")
                .help("Executable name; defaults to project name without -cli"),
        );
    }
    for (name, short, help) in [
        (
            "force",
            Some('f'),
            "Overwrite generated files in a nonempty directory",
        ),
        ("dry-run", None, "List generated files without writing"),
        ("no-git", None, "Skip Git initialization"),
        ("no-lockfile", None, "Skip Cargo lockfile generation"),
    ] {
        let mut arg = Arg::new(name)
            .long(name)
            .action(ArgAction::SetTrue)
            .help(help);
        if let Some(short) = short {
            arg = arg.short(short);
        }
        command = command.arg(arg);
    }
    command
}

pub fn command(mode: Mode) -> Command {
    let mut root = Command::new("rust-scaffold")
        .about("Create and inspect standardized Rust CLI and library projects")
        .disable_version_flag(true)
        .arg(
            Arg::new("version")
                .long("version")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Print the raw version"),
        );
    for (name, library, about) in [
        (
            "cli",
            false,
            "Generate and inspect Rust command-line projects",
        ),
        ("lib", true, "Generate and inspect Rust libraries"),
    ] {
        let mut group = Command::new(name)
            .about(about)
            .subcommand(create(library))
            .subcommand(
                Command::new("inspect")
                    .about("Check a project for standard components")
                    .arg(path()),
            );
        if !library {
            group = group.visible_alias("project");
        }
        root = root.subcommand(group);
    }
    let mut completion = Command::new("completion").about("Generate shell completion scripts");
    for shell in ["bash", "zsh", "fish", "powershell"] {
        completion = completion.subcommand(
            Command::new(shell)
                .about(format!("Generate {shell} completion"))
                .arg(
                    Arg::new("no-descriptions")
                        .long("no-descriptions")
                        .action(ArgAction::SetTrue)
                        .help("Omit completion descriptions"),
                ),
        );
    }
    root = root.subcommand(completion);
    cli_command_kit::setup(root, mode, &help_options())
        .expect("fixed command tree has no skill collision")
}

pub(crate) fn without_descriptions(command: Command) -> Command {
    command
        .about(None::<&'static str>)
        .long_about(None::<&'static str>)
        .mut_args(|arg| {
            arg.help(None::<&'static str>)
                .long_help(None::<&'static str>)
        })
        .mut_subcommands(without_descriptions)
}
