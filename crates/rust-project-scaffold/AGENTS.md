---
created_on: 2026-10-14 13:17
last_modified: 2026-10-14 23:31
status: current
---

# Rust project generation

This internal workspace package belongs to `rust-scaffold-cli`. Inherit `../../AGENTS.md` and run local recipes from this package directory.

## Commands

- Format: `just fmt`; verify formatting: `just fmt-check`.
- Lint: `just lint`.
- Test: `just test`; documentation examples: `cargo test --doc --locked`.
- Coverage: `just coverage`.
- All checks: `just check`.
- Library build: `just build`.

## Contracts

- Keep the library independent of CLI output and argument parsing. Consumers use `Options`, `Plan`, `Generated`, and typed errors directly.
- Generated Rust code lives in `templates/`; `templates/common/` supplies shared project assets. Include hidden workflow and ignore files when inspecting the embedded tree. MiniJinja renders content and relative paths with strict undefined variables and preserved final newlines.
- Verify templates by generating both project kinds and running the actual Cargo toolchain. Generated-project tests use the repository-local `.tmp/generated-rust-target`; preserve bounded shared build storage and isolate these child builds from coverage instrumentation.
- Preserve native filesystem paths. Validate Cargo package/binary names separately from native target paths. Crate names are ASCII Rust identifiers; retain explicit overrides instead of silently transforming keywords. Serialize TOML metadata through the TOML library.
- Inspection uses native TOML and Rust AST parsers. Recognize executable syntax rather than comments or source-string lookalikes. Keep convention checks distinct from type checking and macro expansion.
- Publication uses retained no-follow directory handles. Never replace this with ambient path checks followed by ambient writes. Restore originals on failure, and retain recoverable originals if restoration fails.
- Publication is not crash atomic across files. Preserve the `Error::Published` outcome after successful writes or failed rollback; a caller must distinguish it from an error that leaves no generated files.
- Generated CLI starters embed their full operational skill verbatim. Keep that established generated-project contract, its help alerts, and its interface drift tests synchronized.
- Runtime behavior changes require corresponding behavioral test changes and at least 90% line coverage; `scripts/` is excluded. Record red/green and genuine implementation-disable failures before restoration.

## Boundaries

- Always run project-scoped formatting. Preserve source licenses and dependency provenance in `DEPENDENCIES.md`.
- Never edit the original Go source repository when changing Rust behavior or templates.
- Never publish releases, tags, packages, or production deployments without explicit user authorization.
- Persist standing rules in this file; keep progress, handoffs, and verification logs in `.tmp/` or the conversation.
