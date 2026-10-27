---
created_on: 2026-10-14 13:17
last_modified: 2026-10-14 23:31
status: current
---

`rust-project-scaffold` generates Rust CLI and library starter projects and inspects Cargo projects against their required files and conventions.

# What It Does

- **Prepare before writing:** Render a complete file plan and review its paths without creating a project.
- **Create usable starters:** Include Rust code, tests, documentation, task recipes, licensing, and GitHub workflows.
- **Protect existing work:** Reject nonempty targets by default; forced generation preserves unrelated files and restores replaced originals when publication fails.
- **Inspect conventions:** Report individual checks and a score from zero to ten without modifying the project.

# How It Works

1. Choose a CLI or library, target directory, and project identity.
2. Render a plan, then publish it or use asynchronous generation to also initialize Git and generate Cargo.lock.
3. Read the created paths and any nonfatal Git or lockfile warnings. Inspect an existing directory separately when needed.

# How it Really Works

1. Rendering validates Cargo package, binary, and crate names, then renders MiniJinja templates and their relative paths in memory. TOML metadata uses the native TOML serializer. Crate names are ASCII Rust identifiers; an explicit override allows a Cargo package name such as `for` to use the crate name `for_tool`. Rendering with an explicit identity performs no network requests or writes.
2. Publication refuses symlink destinations and ancestors. It retains open directory handles so replacing a prepared directory with a symlink cannot redirect the writes. On Unix, replacements preserve ownership, permissions, and readable extended attributes; preservation errors abort publication.
3. A failed publication restores replaced files and removes files it created. If restoration fails, the error reports a recovery directory containing the original. Publication replaces file inodes, so existing hard links continue pointing to the previous contents. Multi-file publication is not a crash-atomic filesystem transaction.
4. Keep exclusive control of the target during generation. Directory handles confine writes but do not merge concurrent edits. Synchronous publication and inspection block their caller. Asynchronous generation runs rendering and publication on Tokio's blocking pool and owns its Git/Cargo subprocesses.
5. Cancellation during publication rolls back its writes. Cancellation after publication reports `Error::Published`; files remain. Cancel the supplied token and await the result for an observable final outcome. Dropping generation signals its blocking work to cancel cooperatively; it cannot report a recovery outcome to the dropped caller.
6. Git initialization and `cargo generate-lockfile` run by default. Identity detection consults GitHub CLI, Git configuration, and the current user only when a value is needed. GitHub identity lookup and lockfile generation can contact the network. Dry runs still resolve missing identity values but write no files or run initialization hooks.
7. Inspection parses Cargo.toml and Rust syntax with `toml` and `syn`. It recognizes imported `std::env::var("AGENT")` or `var_os` calls, including aliases, and Clap commands or Parser derives. CLI checks also look for hierarchical help templates and an embedded skill file. It skips invalid Rust files and test-only implementations for CLI support checks. These are syntax conventions: inspection does not type-check, expand macros generally, resolve lexical shadowing, or execute the project.

# Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.98 or newer for the local package.
- Generated projects default to Rust 1.85 and edition 2024; edition 2021 is also supported.
- [Git](https://git-scm.com/) for optional initialization and identity detection; [GitHub CLI](https://cli.github.com/) for optional GitHub identity lookup.
- [just](https://just.systems/) to run generated project recipes.

# Installation

This unpublished implementation crate belongs to the `rust-scaffold-cli` repository. From a consumer beside that repository:

```sh
cargo add rust-project-scaffold --path ../rust-scaffold-cli/crates/rust-project-scaffold
```

# Quick Start

```rust
use rust_project_scaffold::{Identity, Options, Plan, ProjectKind};

fn main() -> rust_project_scaffold::Result<()> {
    let mut options = Options::new("demo-cli", ProjectKind::Cli);
    options.repository = "https://github.com/example/demo".into();
    let identity = Identity {
        owner: "example".into(),
        author: "Example Author".into(),
    };
    let plan = Plan::render(options, &identity)?;
    assert!(plan.files.contains_key(std::path::Path::new("Cargo.toml")));
    Ok(())
}
```

# API

| Entry point | Behavior |
| :--- | :--- |
| `Options::new(target, kind)` | Set project type and defaults, including local year/date. |
| `Plan::render(options, identity)` | Validate inputs and return rendered bytes keyed by relative native paths. |
| `Plan::publish(&token)` | Publish synchronously with rollback; omit Git and Cargo hooks. |
| `generate(options, &token).await` | Detect missing identity, publish, and run enabled hooks. |
| `generate_with(options, &programs, &token).await` | Use selected Git, GitHub CLI, and Cargo executables and a per-process timeout. |
| `inspect(target, kind)` | Return checks, Cargo package/repository/rust-version/edition metadata, issues, and the computed score. |
| `Identity::detect(&programs, owner_needed, author_needed, &token).await` | Resolve only requested identity fields. |

# Configuration

| Option | Default | Meaning |
| :--- | :--- | :--- |
| `name` | Target basename | ASCII project name; supply explicitly for a target basename outside UTF-8. |
| `binary` | Name without a trailing `-cli` | CLI executable name. |
| `package` | Project name with `-` replaced by `_` | ASCII Rust crate identifier for both project kinds, excluding keywords and `_`. |
| `repository`, `author` | Detected identity | HTTP(S) repository URL without credentials, query, fragment, or whitespace; copyright author. |
| `description` | Derived from kind and name | Project description; metadata is serialized as TOML strings. |
| `rust_version`, `edition` | `1.85`, `2024` | Numeric Rust version at least 1.85; edition `2021` or `2024`. |
| `force`, `dry_run` | `false` | Allow overwriting planned files, or return a plan without writing. |
| `init_git`, `run_lockfile` | `true` | Initialize `main` when `.git` is absent; run `cargo generate-lockfile`. |
| `Programs::timeout` | 90 seconds | Deadline for each identity or initialization subprocess. |

# License

[MIT](./LICENSE).
