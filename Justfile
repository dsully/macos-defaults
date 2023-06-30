default: build test

build:
    @cargo build --all

check:
    @cargo check --all

format:
    @cargo fmt --all

lint:
    @cargo clippy --all -- -D clippy::dbg-macro -D warnings

test:
    @cargo test --all
