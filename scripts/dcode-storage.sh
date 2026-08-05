#!/bin/sh

set -eu

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)"
repo_root="$(dirname -- "$script_dir")"
cargo_manifest="$repo_root/codex-rs/Cargo.toml"

[ -d "$repo_root/.git" ] || {
  printf 'dcode storage: expected repository root at %s\n' "$repo_root" >&2
  exit 1
}
[ -f "$cargo_manifest" ] || {
  printf 'dcode storage: missing %s\n' "$cargo_manifest" >&2
  exit 1
}

size_of() {
  if [ -e "$1" ]; then
    du -sh "$1" 2>/dev/null | awk '{print $1}'
  else
    printf '0B\n'
  fi
}

report() {
  bazel_output_base=""
  if command -v bazel >/dev/null 2>&1; then
    bazel_output_base="$(cd "$repo_root" && bazel info output_base 2>/dev/null || true)"
  fi
  printf 'Cargo target: %s (%s)\n' "$repo_root/codex-rs/target" "$(size_of "$repo_root/codex-rs/target")"
  if [ -n "$bazel_output_base" ] && [ -d "$bazel_output_base" ]; then
    printf 'Bazel cache:  %s (%s)\n' "$bazel_output_base" "$(size_of "$bazel_output_base")"
  else
    printf 'Bazel cache:  unavailable\n'
  fi
  printf 'Free space:   '
  df -h "$repo_root" | awk 'NR == 2 {print $4}'
}

case "${1:-report}" in
  report)
    report
    ;;
  clean-cargo)
    printf 'Cleaning Cargo artifacts under %s\n' "$repo_root/codex-rs/target"
    cargo clean --manifest-path "$cargo_manifest"
    report
    ;;
  clean-bazel)
    command -v bazel >/dev/null 2>&1 || {
      printf 'dcode storage: bazel is required\n' >&2
      exit 1
    }
    printf 'Expunging this workspace Bazel cache asynchronously\n'
    (cd "$repo_root" && bazel clean --expunge_async)
    report
    ;;
  clean-all)
    "$0" clean-cargo
    "$0" clean-bazel
    ;;
  *)
    printf 'usage: %s [report|clean-cargo|clean-bazel|clean-all]\n' "$0" >&2
    exit 2
    ;;
esac
