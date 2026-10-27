---
name: rust-scaffold
description: Generate and inspect Rust CLI and library projects with rust-scaffold.
author: alexgorbatchev
metadata:
  created_on: 2026-10-14 13:34
  last_modified: 2026-10-14 23:35
  status: current
---

Read this guide before operating the tool. Use `AGENT=1` for compact output.

## Execution

Confirm the intended target and project kind. Preview with `--dry-run` when reviewing generated paths, then run create with the same options. Inspect the result using the matching `cli inspect` or `lib inspect` command. Inspection reports components; use the generated project's `just check` for compilation and behavioral verification. After exit 6, inspect generated files and reported recovery backups before retrying; preserve those backups until restoration is verified.

`rust-scaffold` creates standalone Rust CLI and library projects with Cargo manifests, Rust source and tests, documentation, and automation. Rust artifacts are unpublished. Generation embeds its templates and this guide, and needs no repository files at runtime.

Supply `--repository https://github.com/example/project --author 'Your Name'` for deterministic offline identity. Otherwise owner discovery tries `gh api user -q .login`, Git configuration, then the system user; author discovery tries Git configuration then the system real name/user. GitHub identity discovery may use network and configured authentication. Optional `git init -b main` and `cargo generate-lockfile` run after files are published. Lockfile generation can download registry metadata. Use `--no-git --no-lockfile` to skip both hooks.

Generation requires exclusive control of the target directory. `--force` replaces only generated paths and preserves unrelated files. Validation and staging precede publication; ordinary failures roll back changes. Files are committed individually; process crashes are not an atomic multi-file transaction. Failed rollback keeps recovery backups in `.rust-scaffold-*` directories and reports their paths. Preserve these for recovery. Cancellation after publication and output failures after publication report that generated files remain.

Stdout holds help, results, completion scripts, and this guide; stderr prints one returned error. Home paths display as `~` in both modes; native paths remain unchanged for filesystem access. Exit statuses: 0 success (including inspection issues and hook warnings), 1 runtime/validation/output failure, 2 argument parsing failure, 6 published files remain after failure, 130 cancellation before publication. `--version` prints only the raw Rust package version plus newline.

## Commands

- `rust-scaffold`: show root help. Root `--version` bool default false prints the raw version. `--help` / `-h` bool default false shows help on every command. `--version` is global.
- `rust-scaffold cli`: show CLI project commands; alias `project`.
- `rust-scaffold cli create [path]`: create a Rust CLI project; aliases `init`, `new`. Optional path defaults to `.`.
- `rust-scaffold cli inspect [path]`: inspect CLI standard components; optional path defaults to `.`.
- `rust-scaffold lib`: show library project commands.
- `rust-scaffold lib create [path]`: create a Rust library; aliases `init`, `new`. Optional path defaults to `.`.
- `rust-scaffold lib inspect [path]`: inspect library standard components; optional path defaults to `.`.
- `rust-scaffold skill`: print this complete guide verbatim, offline, identically in both modes. Takes no arguments.
- `rust-scaffold completion`: show shell choices.
- `rust-scaffold completion bash`, `rust-scaffold completion zsh`, `rust-scaffold completion fish`, `rust-scaffold completion powershell`: print the native shell script. `--no-descriptions` bool default false omits suggestions' descriptions.
- `rust-scaffold help [COMMAND]...`: show help for a command path; native generated help query paths are listed below.

## Create options

Both create commands accept these flags. Empty strings select the resolved default.

| Flag | Type | Default | Contract |
| --- | --- | --- | --- |
| `--name` / `-n` | string | empty | Target directory name; one ASCII alphanumeric, hyphen, underscore segment. |
| `--repository` / `-r` | string | empty | `https://github.com/<owner>/<name>`; HTTP or HTTPS URL with host, without credentials, query, fragment, whitespace, or controls. |
| `--description` / `-d` | string | empty | `A command-line tool for <name>` or `A Rust library for <name>`; NUL rejected. |
| `--author` / `-a` | string | empty | Git configuration or system identity; single line. |
| `--rust-version` | string | empty | `1.85`; numeric two- or three-component minimum Rust version, at least 1.85, without prerelease/build suffixes or leading-zero components. |
| `--edition` | string | empty | `2024`; accepts `2021` or `2024`. Both templates require Rust 1.85 or newer. |
| `--binary` / `-b` | string | empty | CLI create only; project name without final `-cli`; single path segment excluding Cargo's reserved `deps`, `examples`, `build`, and `incremental` names. |
| `--crate-name` / `-p` | string | empty | Both project kinds; project name with hyphens replaced by underscores. ASCII Rust identifier excluding keywords, raw identifiers, and `_`. |
| `--force` / `-f` | bool | false | Permit a nonempty target, overwrite generated files. |
| `--dry-run` | bool | false | Render and list paths; perform no filesystem writes or hooks. Identity discovery still runs if needed. |
| `--no-git` | bool | false | Skip repository initialization. Existing `.git` skips initialization. |
| `--no-lockfile` | bool | false | Skip `cargo generate-lockfile`. |

## Output

Agent create output: `status`, `type` (`cli`/`lib`), `project`, `binary` or `crate_name`, `repository`, `rust_version`, `edition`, `target_dir`, `dry_run`, `files_created`, repeated `file`. Dry-run file count describes the plan. Failed hooks add `warnings_count` and repeated `warning`; generation still succeeds.

Agent inspect output: `status`, `type`, `target_dir`, `score` (one decimal, 0–10), boolean `has_*` checks, optional `package`, `repository`, `rust_version`, and `edition`, `issues_count`, repeated `issue`. Inspection reads syntax and component presence; it does not compile or execute the inspected project. Issues return success for reporting workflows.

Both kinds report `has_cargo_manifest`, `has_justfile`, `has_gitignore`, `has_ci_workflow`, `has_license`, `has_readme`, and `has_agents_doc`. CLI reports also include `has_release_workflow`, `has_agent_mode`, `has_clap`, `has_tree_help`, and `has_skill`. Library reports also include `has_rust_source` and `has_rust_test`.

## Examples

```sh
AGENT=1 rust-scaffold cli create ./demo-cli --repository https://github.com/example/demo --author 'Local Developer' --no-git --no-lockfile
AGENT=1 rust-scaffold cli inspect ./demo-cli
AGENT=1 rust-scaffold lib create ./math-utils --crate-name mathutils --repository https://github.com/example/mathutils --author 'Local Developer' --dry-run
rust-scaffold completion zsh > rust-scaffold.zsh
```

## Native help query paths

Invoke these paths directly to show the selected command's help. `[COMMAND]...` denotes the optional remaining command path accepted by a generated help command. Help queries perform no writes.

```text
rust-scaffold cli help [COMMAND]...
rust-scaffold cli help create
rust-scaffold cli help help
rust-scaffold cli help inspect
rust-scaffold lib help [COMMAND]...
rust-scaffold lib help create
rust-scaffold lib help help
rust-scaffold lib help inspect
rust-scaffold completion help [COMMAND]...
rust-scaffold completion help bash
rust-scaffold completion help fish
rust-scaffold completion help help
rust-scaffold completion help powershell
rust-scaffold completion help zsh
rust-scaffold help cli
rust-scaffold help cli create
rust-scaffold help cli inspect
rust-scaffold help lib
rust-scaffold help lib create
rust-scaffold help lib inspect
rust-scaffold help completion
rust-scaffold help completion bash
rust-scaffold help completion fish
rust-scaffold help completion powershell
rust-scaffold help completion zsh
rust-scaffold help help
rust-scaffold help skill
```
