---
created_on: 2026-10-14 13:17
last_modified: 2026-10-14 23:31
status: current
---

# Dependency and source provenance

The project retains Alex Gorbatchev's original MIT copyright in `LICENSE`. Its generation, inspection, publication, and process capabilities originate from [Go Scaffold](https://github.com/alexgorbatchev/go-scaffold-cli); this version generates native Rust projects. Rust package artifacts are unpublished.

| Dependency | Native use | License |
| :--- | :--- | :--- |
| `cap-std` / `cap-fs-ext` 4.0.3 | Directory capabilities, no-follow traversal, native file replacement and rollback | MIT OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception |
| `uuid` 1.28 | Random private transaction directory names | MIT OR Apache-2.0 |
| `xattr` 1.6 | Preserve readable Unix extended attributes using file descriptors | MIT OR Apache-2.0 |
| `minijinja` 3 | Render native templates and relative paths with strict undefined variables | Apache-2.0 |
| `toml` 1 | Parse Cargo manifests and serialize metadata string literals | MIT OR Apache-2.0 |
| `syn` 2 | Parse Rust syntax, identifiers, imports, calls, attributes, and macros | MIT OR Apache-2.0 |
| `semver` 1 | Validate numeric Rust minimum versions | MIT OR Apache-2.0 |
| `url` 2 | Parse HTTP(S) repository URLs | MIT OR Apache-2.0 |
| `include_dir` 0.7 | Embed all template assets, including hidden files | MIT |
| `chrono` 0.4 | Local timestamps and copyright year | MIT OR Apache-2.0 |
| `whoami` 2.1 | Native username and author fallbacks | MIT OR Apache-2.0 OR BSL-1.0 |
| `tokio` / `tokio-util` | Owned asynchronous subprocesses, blocking work, cancellation tokens | MIT |
| `walkdir` 2 | Inspect Rust files without following directory symlinks | MIT OR Unlicense |
| `thiserror` 2 | Typed public failures | MIT OR Apache-2.0 |
| `libc` 0.2 | Unix process-group termination and descriptor ownership preservation | MIT OR Apache-2.0 |
| `tempfile` 3 | Native isolated test directories | MIT OR Apache-2.0 |

The generated CLI uses native Clap and clap_complete APIs. Its project contains its own output helpers, recursive help, tests, and embedded operational skill; it has no dependency on this migration workspace. Generated workflow publication requires an explicitly configured remote and consumer action; local generation performs no GitHub writes.

Native API references: [MiniJinja syntax configuration](https://docs.rs/minijinja/latest/minijinja/syntax/), [Cargo manifests](https://doc.rust-lang.org/cargo/reference/manifest.html), [Rust crate identifiers](https://doc.rust-lang.org/reference/identifiers.html), [syn parsing](https://docs.rs/syn/latest/syn/fn.parse_file.html), [Cargo lockfile generation](https://doc.rust-lang.org/cargo/commands/cargo-generate-lockfile.html). Binary names `deps`, `examples`, `build`, and `incremental` are rejected because [Cargo's native validation](https://github.com/rust-lang/cargo/blob/master/src/util/restricted_names.rs) reserves those artifact directories; other Cargo package names may use an explicit valid crate identifier.

## Correctness boundaries

Force replacement changes file inodes and does not preserve hard-link sharing. Unix metadata preservation covers accessible extended attributes; privileged attributes that the operating system does not reveal cannot be copied. The project requires exclusive control of the target while generating. Runtime platform behavior requires native execution; a Linux check does not establish Windows or macOS filesystem/process semantics.

Rust source inspection parses syntax and imports rather than comments or string lookalikes. It remains a convention check: it does not resolve type information, lexical shadowing, inherited workspace dependency metadata, or arbitrary macro expansion. Invalid Rust files do not satisfy syntax checks. Platform-conditional dependencies are recognized as declarations without evaluating their target predicates.
