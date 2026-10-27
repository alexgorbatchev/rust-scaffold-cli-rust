`rust-scaffold` creates standardized Rust CLI and library projects and checks existing projects for missing components. It is for Rust developers who want tests, documentation, and automation ready in the project directory.

# What It Does

- **Generate projects:** Create a Rust command-line tool or reusable library from embedded templates.
- **Inspect standards:** Report missing project files, tests, and CLI integration components with a score out of ten.
- **Preview safely:** List planned files without writing them, or deliberately replace generated files while retaining unrelated files.

# How It Works

- Choose `cli create` or `lib create` and a destination; the current directory is the default.
- Supply project details or let the tool derive names and identity.
- Review the generated files or use `inspect` to see which standard components are present.

# How it Really Works

- Templates and the agent guide are embedded; generation and `skill` work without the source checkout. The executable is Rust and its generated projects are Rust.
- Owner discovery may query GitHub using configured `gh` authentication. Explicit `--repository` and `--author` avoid identity probes. Git initialization and `cargo generate-lockfile` run after generation unless skipped; lockfile generation can download registry metadata.
- `--force` replaces generated paths and keeps unrelated files. Ordinary publication failures roll back changes. Failed restoration retains recovery backups and reports their location. Keep exclusive control of the target during generation; separate file replacements are not crash-atomic.
- `AGENT=1` prints compact key-value results; ordinary mode prints summaries and next steps. Results go to stdout and returned errors appear once on stderr. Displayed home paths use `~`.
- Exit 0 includes inspection issues and hook warnings; 1 means runtime or validation failure, 2 argument parsing failure, 6 means generated files remain after failure, and 130 means cancellation before publication. An output failure after generation also exits 6.
- Inspection reads files and Rust syntax; it does not execute or type-check the inspected project. This checkout's runtime verification is on Linux.

# Installation

Rust release artifacts are unpublished. The original Go project's release archives contain a different implementation. Contributor build instructions are in [AGENTS.md](AGENTS.md).

# Setup

- [Git](https://git-scm.com/) initializes generated repositories and supplies optional identity. Use `--no-git` to skip initialization.
- [Rust 1.85 or newer](https://rust-lang.org/tools/install/) runs the default `cargo generate-lockfile` hook and generated projects. Use `--no-lockfile` to skip the hook.
- [GitHub CLI](https://cli.github.com/) optionally discovers the repository owner from an authenticated account; explicit `--repository` avoids it. No GitHub account is needed when repository and author are explicit.

# Quick Start

```sh
rust-scaffold cli create ./demo-cli --repository https://github.com/example/demo --author 'Local Developer' --no-git --no-lockfile
AGENT=1 rust-scaffold cli inspect ./demo-cli
rust-scaffold lib create ./math-utils --crate-name mathutils --repository https://github.com/example/mathutils --author 'Local Developer' --dry-run
```

Sample Output (inspection excerpt):

```text
status: ok
type: cli
score: 10.0
```

The target is printed as an absolute path, with the home directory folded to `~`; its value depends on the destination.

# Options & Flags

Global options:

| Flag | Short | Default | Description |
| :--- | :--- | :--- | :--- |
| `--help` | `-h` | `false` | Show help for the selected command |
| `--version` | | `false` | Print the raw version |

`rust-scaffold cli create [path]` and `rust-scaffold lib create [path]` (`path` defaults to `.`):

| Flag | Short | Default | Description |
| :--- | :--- | :--- | :--- |
| `--name <name>` | `-n` | target directory name | Project name |
| `--repository <url>` | `-r` | `https://github.com/<owner>/<name>` | HTTP or HTTPS repository URL |
| `--description <text>` | `-d` | description derived from name | Project documentation description |
| `--author <name>` | `-a` | Git or system identity | Copyright author |
| `--rust-version <version>` | | `1.85` | Numeric minimum Rust version, at least 1.85 |
| `--edition <edition>` | | `2024` | Rust edition: `2021` or `2024` |
| `--binary <name>` | `-b` | project name without `-cli` | CLI create only: executable name |
| `--crate-name <name>` | `-p` | project name with hyphens replaced by underscores | ASCII Rust crate identifier for either project kind |
| `--force` | `-f` | `false` | Replace generated files in a nonempty directory |
| `--dry-run` | | `false` | List planned files without writing or running hooks |
| `--no-git` | | `false` | Skip Git initialization |
| `--no-lockfile` | | `false` | Skip `cargo generate-lockfile` |

String flags have empty parser defaults; empty values select the resolved defaults shown above.

`rust-scaffold completion bash|zsh|fish|powershell`:

| Flag | Short | Default | Description |
| :--- | :--- | :--- | :--- |
| `--no-descriptions` | | `false` | Omit completion descriptions |

# Agent Guide

Run `AGENT=1 rust-scaffold skill` to read the complete embedded usage guide offline. The canonical local guide is [skills/rust-scaffold/SKILL.md](skills/rust-scaffold/SKILL.md); publication of this Rust project's skill is pending.

# License

[MIT License](LICENSE) (c) 2026 Alex Gorbatchev.
