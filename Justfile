default: build test

build:
    @cargo build --all

check:
    @cargo check --all

format:
    @cargo fmt --all

format-check:
    @cargo fmt --all -- --check

lint:
    @cargo clippy --all -- -D clippy::dbg-macro -D warnings

test:
    @cargo test --all

patch:
    @cargo release version patch --execute

minor:
    @cargo release version minor --execute

major:
    @cargo release version major --execute

udeps:
    RUSTC_BOOTSTRAP=1 cargo +nightly udeps --all-targets --backend depinfo
