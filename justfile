default:
  @just --list

# `just` (haha, get it?) build the dbg binary
build:
  cargo build

# full check before committing
check:
  cargo fmt --all
  cargo clippy -- -D warnings
  cargo test

# tag `Cargo.toml` version & push
tag:
  #!/usr/bin/env bash
  set -euo pipefail
  ver=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)
  [[ -n "$ver" ]] || { echo "no version in Cargo.toml"; exit 1; }
  tag="v$ver"
  git diff --quiet && git diff --cached --quiet || { echo "commit changes first"; exit 1; }
  git rev-parse "$tag" >/dev/null 2>&1 && { echo "tag $tag already exists"; exit 1; }
  git tag -a "$tag" -m "release $tag"
  git push origin "refs/tags/$tag"
  echo "pushed $tag — watch Actions → Release"

install:
  cargo install --path . --locked
