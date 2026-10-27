---
created_on: 2026-10-14 13:34
last_modified: 2026-10-14 23:21
status: current
---

# rust-scaffold-cli

Rust command-line consumer of the internal `crates/rust-project-scaffold` package in this repository's Cargo workspace. It generates standalone Rust projects with native Cargo manifests.

## Development

- Rust 1.98+, Cargo, rustfmt, Clippy, just, and cargo-llvm-cov with matching LLVM tools are required for contributor checks.
- Build: `just build` writes `bin/rust-scaffold`.
- Run: `just run <args>` or `just run-ai <args>`.
- Format: `just fmt`; check: `just check`; focused tests: `cargo test --test workflows`.
- Use the inherited migration build locks and storage budget; preserve unfinished development caches. Scope formatting to this package.
- All CLI tests are offline. Supply explicit repository/author and skip hooks, or select controlled native subprocesses through the library's `Programs`; never use real GitHub identity or dependency downloads in tests.

## Contracts

- Keep argument parsing and output in this package; generation, identity, processes, publication, and inspection belong to the internal scaffold library. Shared help and path/message folding belong to `cli-command-kit`.
- Preserve `cli`/`lib` create and inspect, `project`/`init`/`new` aliases, optional current-directory paths, and four native completion commands. Do not invent unrelated commands.
- Generate Rust projects using Cargo metadata and Rust syntax. Keep repository URLs, minimum Rust versions, editions, crate identifiers, and executable names distinct. The generated projects must be standalone and use published dependencies rather than this migration's sibling paths.
- Keep `skills/rust-scaffold/SKILL.md` synchronized with every command, argument, flag, shorthand, parser type, default, environment, output, write state, and exit status. Embed and print this full document verbatim: the source project's full-guide contract overrides the generic install-pointer convention. Maintain coverage of the live native help query paths as well.
- All agent help starts with the shared skill alert. Raw version output stays version plus newline. Home folding is display-only; use native paths for all filesystem operations.
- Validate explicit create values before identity subprocesses. Print returned errors once at the entrypoint; stdout holds results. Inspection issues and post-generation hook warnings exit zero.
- Generation requires exclusive target ownership. Preserve recovery backups when rollback fails. Report published state for cancellation or output failure after files are created; avoid implying that retry is safe.
- Generate completion into memory before the checked external write: the resolved fish generator can panic on a failing writer even through `try_generate`.
- Keep existing-Git reports factual: initialization is checked/skipped when `.git` exists, rather than claiming a new repository was initialized.

## Boundaries

- Always change a behavioral test when runtime results change and maintain at least 90% line coverage; `scripts/` is excluded. Use red/green and genuine implementation-disable/restoration evidence.
- Never modify the original Go source, touch credentials, make live service writes, or publish releases, tags, packages, or deployments without explicit user authorization.
- Rust artifacts have no published release channel. Do not point installation instructions at original Go release archives.
- Persist standing rules here in place; keep progress, test results, and handoffs outside agent instructions.
