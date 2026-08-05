#!/bin/sh

set -eu

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)"
repo_root="$(dirname -- "$script_dir")"
mode="${1:---check}"

[ -d "$repo_root/.git" ] || {
  printf 'dcode sync: expected repository root at %s\n' "$repo_root" >&2
  exit 1
}

cd "$repo_root"
if [ -n "$(git status --porcelain)" ]; then
  printf 'dcode sync: working tree must be clean\n' >&2
  exit 1
fi

git remote get-url upstream >/dev/null 2>&1 || {
  printf 'dcode sync: add the OpenAI remote first:\n' >&2
  printf '  git remote add upstream https://github.com/openai/codex.git\n' >&2
  exit 1
}

git fetch upstream main
ahead="$(git rev-list --count upstream/main..HEAD)"
behind="$(git rev-list --count HEAD..upstream/main)"
printf 'DCode overlay commits: %s; new upstream commits: %s\n' "$ahead" "$behind"

case "$mode" in
  --check)
    git diff --stat upstream/main...HEAD
    ;;
  --apply)
    git rebase upstream/main
    printf 'Upstream rebase complete. Run scripts/check-downstream-boundary.py upstream/main.\n'
    ;;
  *)
    printf 'usage: %s [--check|--apply]\n' "$0" >&2
    exit 2
    ;;
esac
