#!/usr/bin/env bash
# options (pass after `--` when piping):
#   --version v0.2.0   pin a tag (default: latest)
#   --dir PATH         install dir (default: ~/.local/bin, or $SHEIKAH_INSTALL_DIR)
#   --print-path       print install path of sheikah and exit
set -euo pipefail

REPO="${SHEIKAH_REPO:-vahrina/sheikah}"
BIN=sheikah
TAG=""
DIR="${SHEIKAH_INSTALL_DIR:-$HOME/.local/bin}"

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}
need() { command -v "$1" >/dev/null 2>&1 || die "'$1' is required"; }

while [[ $# -gt 0 ]]; do
  case "$1" in
  --version | -v)
    TAG="${2:?}"
    shift 2
    ;;
  --dir | -d)
    DIR="${2:?}"
    shift 2
    ;;
  --print-path)
    command -v "$BIN" || die "$BIN not on $PATH"
    command -v "$BIN"
    exit 0
    ;;
  -h | --help)
    sed -n '2,11p' "$0"
    exit 0
    ;;
  *) die "unknown option: $1" ;;
  esac
done

need curl
need tar
need uname
need mktemp

os=$(uname -s)
machine=$(uname -m)
case "$os:$machine" in
  Linux:x86_64 | Linux:amd64) arch=x86_64 ;;
  Linux:aarch64 | Linux:arm64) arch=aarch64 ;;
  *) die "linux only (got $os $machine); build from source instead" ;;
esac

api="https://api.github.com/repos/$REPO/releases"
if [[ -z "$TAG" ]]; then
  TAG=$(curl -fsSL "$api/latest" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)
  [[ -n "$TAG" ]] || die "no release found for $REPO (has a tag been pushed yet?)"
elif [[ "$TAG" != v* ]]; then
  TAG="v$TAG"
fi

base="https://github.com/$REPO/releases/download/$TAG"
archive="sheikah-${TAG}-${arch}.tar.gz"
tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

printf 'downloading %s ...\n' "$archive" >&2
curl -fsSL "$base/$archive" -o "$tmpdir/$archive"
if curl -fsSL "$base/$archive.sha256" -o "$tmpdir/$archive.sha256" 2>/dev/null; then
  want=$(awk '{print $1}' "$tmpdir/$archive.sha256")
  if command -v sha256sum >/dev/null 2>&1; then
    got=$(sha256sum "$tmpdir/$archive" | awk '{print $1}')
  else
    need shasum
    got=$(shasum -a 256 "$tmpdir/$archive" | awk '{print $1}')
  fi
  [[ "$got" == "$want" ]] || die "checksum mismatch for $archive"
  printf 'checksum ok\n' >&2
fi

tar -xzf "$tmpdir/$archive" -C "$tmpdir"
[[ -f "$tmpdir/$BIN" ]] || die "archive missing $BIN"

mkdir -p "$DIR"
install -m 755 "$tmpdir/$BIN" "$DIR/$BIN"

case ":$PATH:" in
*":$DIR:"*) ;;
*)
  printf '\nnote: %s is not on $PATH. add this to your shell rc:\n' "$DIR" >&2
  printf '  export PATH="%s:$PATH"\n' "$DIR" >&2
  ;;
esac

ver=$("$DIR/$BIN" -V 2>/dev/null || true)
printf 'installed %s → %s/%s\n' "${ver:-$TAG}" "$DIR" "$BIN" >&2
printf 'run again later to update\n' >&2
