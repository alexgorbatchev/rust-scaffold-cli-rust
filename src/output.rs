use cli_command_kit::{Mode, display_message, display_path};
use rust_project_scaffold::{Generated, Inspection, ProjectKind};

pub(crate) fn generated(result: &Generated, mode: Mode) -> String {
    let options = &result.options;
    let target = display_path(&options.target);
    let warnings: Vec<_> = result.warnings().collect();
    let mut output = if mode == Mode::Agent {
        format!(
            "status: ok\ntype: {}\nproject: {}\n{}: {}\nrepository: {}\nrust_version: {}\nedition: {}\ntarget_dir: {}\ndry_run: {}\nfiles_created: {}\n",
            options.kind.as_str(),
            options.name,
            if options.kind == ProjectKind::Cli {
                "binary"
            } else {
                "crate_name"
            },
            if options.kind == ProjectKind::Cli {
                &options.binary
            } else {
                &options.package
            },
            options.repository,
            options.rust_version,
            options.edition,
            target,
            options.dry_run,
            result.files.len()
        )
    } else {
        let prefix = if options.dry_run { "[DRY-RUN]" } else { "[OK]" };
        let description = if options.kind == ProjectKind::Cli {
            format!("CLI project {} ({})", options.name, options.repository)
        } else {
            format!(
                "library project {} (crate {}, {})",
                options.name, options.package, options.repository
            )
        };
        format!(
            "{prefix} Created {description} in {target}\n{prefix} Generated {} files:\n",
            result.files.len()
        )
    };
    for file in &result.files {
        output.push_str(&format!(
            "{}{}\n",
            if mode == Mode::Agent {
                "file: "
            } else {
                "  - "
            },
            display_path(file)
        ));
    }
    if mode == Mode::Agent {
        if !warnings.is_empty() {
            output.push_str(&format!("warnings_count: {}\n", warnings.len()));
            for warning in warnings {
                output.push_str(&format!("warning: {}\n", display_message(warning)));
            }
        }
    } else if !options.dry_run {
        for (enabled, name, error) in [
            (
                options.init_git,
                "Git initialization checked",
                &result.git_error,
            ),
            (
                options.run_lockfile,
                "Generated Cargo lockfile",
                &result.lockfile_error,
            ),
        ] {
            if enabled {
                match error {
                    Some(error) => output.push_str(&format!("[WARN] {}\n", display_message(error))),
                    None => output.push_str(&format!("[OK] {name}\n")),
                }
            }
        }
        output.push_str(&format!(
            "\nNext steps in {target}:\n  just check\n  {}\n",
            if options.kind == ProjectKind::Cli {
                "just run system status"
            } else {
                "just test"
            }
        ));
    }
    output
}

pub(crate) fn inspection(result: &Inspection, mode: Mode) -> String {
    let issues: Vec<_> = result.issues().collect();
    let target = display_path(&result.target);
    let mut output = if mode == Mode::Agent {
        format!(
            "status: ok\ntype: {}\ntarget_dir: {target}\nscore: {:.1}\n",
            result.kind.as_str(),
            result.score()
        )
    } else {
        format!(
            "{} Project: {target}\nStandards Score: {:.1} / 10\n\n",
            if result.kind == ProjectKind::Cli {
                "CLI"
            } else {
                "Library"
            },
            result.score()
        )
    };
    for check in &result.checks {
        if mode == Mode::Agent {
            output.push_str(&format!("{}: {}\n", check.name, check.passed));
        } else {
            output.push_str(&format!(
                "  [{}] {}{}",
                if check.passed { "OK" } else { "WARN" },
                if check.passed { "" } else { "Missing " },
                check.label
            ));
            if check.name == "has_cargo_manifest"
                && check.passed
                && let Some(package) = &result.package
            {
                output.push_str(&format!(" ({package})"));
            }
            output.push('\n');
        }
    }
    if mode == Mode::Agent {
        for (key, value) in [
            ("package", &result.package),
            ("repository", &result.repository),
            ("rust_version", &result.rust_version),
            ("edition", &result.edition),
        ] {
            if let Some(value) = value {
                output.push_str(&format!("{key}: {value}\n"));
            }
        }
        output.push_str(&format!("issues_count: {}\n", issues.len()));
    } else if issues.is_empty() {
        output.push_str(&format!(
            "\nAll standard {} components are present and verified!\n",
            if result.kind == ProjectKind::Cli {
                "CLI"
            } else {
                "library"
            }
        ));
    } else {
        output.push_str(&format!("\nIssues found ({}):\n", issues.len()));
    }
    for issue in issues {
        output.push_str(&format!(
            "{}{}\n",
            if mode == Mode::Agent {
                "issue: "
            } else {
                "  - "
            },
            issue
        ));
    }
    output
}
