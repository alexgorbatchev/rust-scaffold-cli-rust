---
created_on: 2026-10-14 13:34
last_modified: 2026-10-14 23:21
status: current
---

# Dependencies and provenance

This Rust CLI preserves the create/inspect workflows and project kinds from [alexgorbatchev/go-scaffold-cli](https://github.com/alexgorbatchev/go-scaffold-cli), including its aliases, output modes, embedded full usage guide, and optional Git and Cargo lockfile hooks. Original source attribution is retained in LICENSE. The native scaffold library owns the Rust templates and their behavior.

| Dependency | Purpose | License |
| --- | --- | --- |
| `rust-project-scaffold` (local) | Actual rendering, safe publication/recovery, native subprocesses/identity, and Rust inspection | MIT |
| `cli-command-kit` (local) | Complete command-tree help, alerts, skill drift validation, and display-only home folding | MIT |
| `clap` 4.6 | Native argument parsing and help routing | MIT OR Apache-2.0 |
| `clap_complete` 4.6 | Native bash/zsh/fish/PowerShell completion | MIT OR Apache-2.0 |
| `tokio` 1 | Async process runtime and Ctrl-C lifecycle | MIT |
| `thiserror` 2 | Typed boundary errors | MIT OR Apache-2.0 |
| `tempfile` 3 (development) | Isolated real filesystem tests | MIT OR Apache-2.0 |

Resolved versions are recorded in Cargo.lock. APIs were checked against dependency sources and official [Clap command documentation](https://docs.rs/clap/4.6.7/clap/struct.Command.html) and [completion documentation](https://docs.rs/clap_complete/4.6.11/clap_complete/). Completion bytes are generated into memory before checked output because some generators' fallible methods delegate to writers that panic on failure.

Behavior differences are intentional: explicit inputs are validated before identity probes; returned errors print once; output failure after publication is distinguished from a pre-publication failure; existing Git repositories are not reported as newly initialized. Native argument errors exit 2. The library rejects invalid Rust version components and preserves recovery backups and native filesystem paths. Rust artifacts and a release channel are unpublished; no original Go release URLs are treated as Rust binaries.
