default:
  @just --list

# `just` (haha, get it?) build the dbg binary
build:
  cargo build

# full check before committing
check:
  cargo fmt --all --check
  cargo clippy -- -D warnings
  cargo test

# run checks & prepare for release
release: check
  cargo build --release

install:
  cargo install --path .
