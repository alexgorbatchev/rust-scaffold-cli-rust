mod rust_project_scaffold 'crates/rust-project-scaffold/justfile'

default:
    @just --list

run *args:
    cargo run --package rust-scaffold-cli --locked --bin rust-scaffold -- {{ args }}

run-ai *args:
    AGENT=1 cargo run --package rust-scaffold-cli --locked --bin rust-scaffold -- {{ args }}

[private]
_fmt:
    cargo fmt --package rust-scaffold-cli

[private]
_fmt-check:
    cargo fmt --package rust-scaffold-cli -- --check

[private]
_lint:
    cargo clippy --package rust-scaffold-cli --all-targets --locked -- -D warnings

[private]
_test:
    mkdir -p .tmp
    cargo test --package rust-scaffold-cli --all-targets --locked

docs:
    cargo test --package rust-scaffold-cli --doc --locked
    RUSTDOCFLAGS='-D warnings' cargo doc --package rust-scaffold-cli --no-deps --locked

coverage:
    mkdir -p .tmp
    cargo llvm-cov --package rust-scaffold-cli --exclude-from-report rust-project-scaffold --all-targets --locked --fail-under-lines 90

check: _fmt-check _lint _test docs coverage
    just rust_project_scaffold check

build:
    cargo build --package rust-scaffold-cli --bin rust-scaffold --locked
    mkdir -p bin
    cp "${CARGO_TARGET_DIR:-target}/debug/rust-scaffold" bin/rust-scaffold

test: _test
    just rust_project_scaffold test

lint: _lint
    just rust_project_scaffold lint

fmt: _fmt
    just rust_project_scaffold fmt

fmt-check: _fmt-check
    just rust_project_scaffold fmt-check
