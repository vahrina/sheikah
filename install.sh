#!/usr/bin/env bash
# run with --help for options
set -euo pipefail

BIN=sheikah
WORK=""

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

need() { command -v "$1" >/dev/null 2>&1 || die "'$1' is required"; }

cleanup() {
  if [[ -n "$WORK" ]]; then rm -rf "$WORK"; fi
}

usage() {
  cat <<'EOF'
usage: curl -fsSL <url-of-this-script> | bash -s -- [options]

  --version v0.2.0   pin a tag
  --dir PATH         install dir (default: ~/.local/bin, or $SHEIKAH_INSTALL_DIR)
  --print-path       print the path of the installed sheikah & exit
  -h, --help         show this help

env: SHEIKAH_REPO (default: vahrina/sheikah), SHEIKAH_INSTALL_DIR
EOF
}

main() {
  local repo="${SHEIKAH_REPO:-vahrina/sheikah}"
  local dir="${SHEIKAH_INSTALL_DIR:-}"
  local tag="" print_path=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
    --version | -v)
      [[ $# -ge 2 ]] || die "$1 needs a value"
      tag="$2"
      shift 2
      ;;
    --dir | -d)
      [[ $# -ge 2 ]] || die "$1 needs a value"
      dir="$2"
      shift 2
      ;;
    --print-path)
      print_path=1
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *) die "unknown option: $1 (see --help)" ;;
    esac
  done

  if [[ -z "$dir" ]]; then
    [[ -n "${HOME:-}" ]] || die "HOME is not set; pass --dir"
    dir="$HOME/.local/bin"
  fi

  if ((print_path)); then
    if [[ -x "$dir/$BIN" ]]; then
      printf '%s\n' "$dir/$BIN"
    elif ! command -v "$BIN"; then
      die "$BIN not found in $dir or on PATH"
    fi
    exit 0
  fi

  [[ "$repo" =~ ^[A-Za-z0-9._-]+/[A-Za-z0-9._-]+$ ]] || die "invalid repo: $repo"

  need curl
  need tar
  need uname
  need mktemp
  need install

  local -a sha
  if command -v sha256sum >/dev/null 2>&1; then
    sha=(sha256sum)
  elif command -v shasum >/dev/null 2>&1; then
    sha=(shasum -a 256)
  else
    die "'sha256sum' or 'shasum' is required"
  fi

  local os machine arch
  os=$(uname -s)
  machine=$(uname -m)
  case "$os:$machine" in
  Linux:x86_64 | Linux:amd64) arch=x86_64 ;;
  Linux:aarch64 | Linux:arm64) arch=aarch64 ;;
  *) die "linux x86_64/aarch64 only (got $os $machine); build from source instead" ;;
  esac

  local -a fetch=(curl -fsSL --proto '=https' --tlsv1.2 --retry 3)

  if [[ -z "$tag" ]]; then
    # resolve "latest" via the redirect, which avoids the rate-limited API
    local url
    url=$("${fetch[@]}" -I -o /dev/null -w '%{url_effective}' \
      "https://github.com/$repo/releases/latest") || url=""
    [[ "$url" == */releases/tag/* ]] ||
      die "no release found for $repo (has a tag been pushed yet?)"
    tag="${url##*/releases/tag/}"
  elif [[ "$tag" != v* ]]; then
    tag="v$tag"
  fi
  [[ "$tag" =~ ^v[0-9A-Za-z._+-]+$ ]] || die "invalid version: $tag"

  local base="https://github.com/$repo/releases/download/$tag"
  local archive="$BIN-$tag-$arch.tar.gz"

  WORK=$(mktemp -d)
  trap cleanup EXIT
  trap 'exit 1' INT TERM

  printf 'downloading %s ...\n' "$archive" >&2
  "${fetch[@]}" -o "$WORK/$archive" "$base/$archive" ||
    die "download failed: $base/$archive"
  "${fetch[@]}" -o "$WORK/$archive.sha256" "$base/$archive.sha256" ||
    die "no checksum published for $archive; refusing to install"

  local want got
  read -r want _ <"$WORK/$archive.sha256" || true
  [[ "$want" =~ ^[0-9A-Fa-f]{64}$ ]] || die "malformed checksum file for $archive"
  got=$("${sha[@]}" "$WORK/$archive")
  got=${got%% *}
  [[ "${got,,}" == "${want,,}" ]] || die "checksum mismatch for $archive"
  printf 'checksum ok (sha256 %s)\n' "${got,}" >&2

  mkdir "$WORK/x"
  tar -xzf "$WORK/$archive" -C "$WORK/x" || die "failed to extract $archive"
  [[ -f "$WORK/x/$BIN" ]] || die "archive missing $BIN"

  mkdir -p "$dir" || die "cannot create $dir"
  local staged="$dir/.$BIN.new.$$"
  install -m 755 "$WORK/x/$BIN" "$staged" || die "cannot write to $dir"
  mv -f "$staged" "$dir/$BIN" || {
    rm -f "$staged"
    die "cannot replace $dir/$BIN"
  }

  case ":$PATH:" in
  *":$dir:"*) ;;
  *)
    printf '\nnote: %s is not on PATH. add this to your shell rc:\n' "$dir" >&2
    # shellcheck disable=SC2016  # literal $PATH is intended in the hint
    printf '  export PATH="%s:$PATH"\n' "$dir" >&2
    ;;
  esac

  local ver
  ver=$("$dir/$BIN" -V 2>/dev/null) || ver=""
  printf 'installed %s -> %s/%s\n' "${ver:-$tag}" "$dir" "$BIN" >&2
  printf 'run again to update\n' >&2
}

main "$@"
