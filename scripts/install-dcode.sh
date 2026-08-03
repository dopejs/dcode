#!/bin/sh

set -eu

REPOSITORY="${DCODE_GITHUB_REPOSITORY:-dopejs/dcode}"
RELEASE="${DCODE_RELEASE:-latest}"
INSTALL_DIR="${DCODE_INSTALL_DIR:-$HOME/.local/bin}"
CODEX_HOME_DIR="${CODEX_HOME:-$HOME/.codex}"
STANDALONE_ROOT="$CODEX_HOME_DIR/packages/dcode-standalone"
RELEASES_DIR="$STANDALONE_ROOT/releases"
CURRENT_LINK="$STANDALONE_ROOT/current"
BIN_PATH="$INSTALL_DIR/dcode"
DOWNLOAD_BASE="${DCODE_RELEASE_BASE_URL:-https://github.com/$REPOSITORY/releases/download}"
tmp_dir=""
stage_dir=""
stage_created=0

die() {
  printf 'dcode installer: %s\n' "$1" >&2
  exit 1
}

cleanup() {
  if [ "$stage_created" -eq 1 ] && [ -n "$stage_dir" ]; then
    case "$stage_dir" in
      "$RELEASES_DIR"/.staging.*) rm -rf -- "$stage_dir" ;;
    esac
  fi
  if [ -n "$tmp_dir" ]; then
    case "$tmp_dir" in
      "${TMPDIR:-/tmp}"/dcode-install.*) rm -rf -- "$tmp_dir" ;;
    esac
  fi
}
trap cleanup EXIT HUP INT TERM

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "$1 is required"
}

download() {
  url="$1"
  output="$2"
  label="$3"
  printf 'Downloading %s\n' "$label"
  if command -v curl >/dev/null 2>&1; then
    curl -fL --progress-bar "$url" -o "$output"
  elif command -v wget >/dev/null 2>&1; then
    wget -O "$output" "$url"
  else
    die "curl or wget is required"
  fi
}

normalize_version() {
  case "$1" in
    dcode-v*) printf '%s\n' "${1#dcode-v}" ;;
    v*) printf '%s\n' "${1#v}" ;;
    *) printf '%s\n' "$1" ;;
  esac
}

validate_version() {
  printf '%s\n' "$1" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.]+)?$' \
    || die "invalid release version: $1"
}

resolve_latest_version() {
  if [ "$DOWNLOAD_BASE" != "https://github.com/$REPOSITORY/releases/download" ]; then
    die "DCODE_RELEASE must be explicit when DCODE_RELEASE_BASE_URL is overridden"
  fi
  require_command curl
  latest_url="$(curl -fsSL -o /dev/null -w '%{url_effective}' \
    "https://github.com/$REPOSITORY/releases/latest")"
  tag="${latest_url##*/}"
  case "$tag" in
    dcode-v*) normalize_version "$tag" ;;
    *) die "latest GitHub release tag is not a dcode-v* tag: $tag" ;;
  esac
}

detect_target() {
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$arch" in
    x86_64 | amd64) arch="x86_64" ;;
    arm64 | aarch64) arch="aarch64" ;;
    *) die "unsupported architecture: $arch" ;;
  esac
  case "$os" in
    Darwin) printf '%s-apple-darwin\n' "$arch" ;;
    Linux) printf '%s-unknown-linux-gnu\n' "$arch" ;;
    *) die "unsupported operating system: $os" ;;
  esac
}

file_sha256() {
  path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$path" | awk '{print $1}'
  elif command -v openssl >/dev/null 2>&1; then
    openssl dgst -sha256 "$path" | sed 's/^.*= //'
  else
    die "sha256sum, shasum, or openssl is required"
  fi
}

replace_symlink() {
  link_path="$1"
  target="$2"
  if [ -e "$link_path" ] && [ ! -L "$link_path" ]; then
    die "refusing to overwrite non-symlink path: $link_path"
  fi
  tmp_link="${link_path}.tmp.$$"
  ln -s "$target" "$tmp_link"
  case "$(uname -s)" in
    Darwin) mv -fh "$tmp_link" "$link_path" ;;
    Linux) mv -fT "$tmp_link" "$link_path" ;;
    *) die "unsupported operating system for symlink replacement" ;;
  esac
}

validate_package() {
  package_dir="$1"
  expected_version="$2"
  [ -x "$package_dir/bin/dcode" ] || die "package does not contain bin/dcode"
  [ -x "$package_dir/bin/codex-code-mode-host" ] || die "package is missing code-mode host"
  [ -x "$package_dir/codex-path/rg" ] || die "package is missing ripgrep"
  "$package_dir/bin/dcode" --version | grep -F "$expected_version" >/dev/null \
    || die "downloaded binary version does not match $expected_version"
}

require_command tar
require_command mktemp

if [ "$RELEASE" = "latest" ]; then
  version="$(resolve_latest_version)"
else
  version="$(normalize_version "$RELEASE")"
fi
validate_version "$version"
target="$(detect_target)"
tag="dcode-v$version"
asset="dcode-package-$target.tar.gz"
release_url="$DOWNLOAD_BASE/$tag"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/dcode-install.XXXXXX")"
archive="$tmp_dir/$asset"
checksums="$tmp_dir/dcode_SHA256SUMS"
download "$release_url/$asset" "$archive" "$asset"
download "$release_url/dcode_SHA256SUMS" "$checksums" "dcode_SHA256SUMS"

expected="$(awk -v asset="$asset" '$2 == asset { print $1; exit }' "$checksums")"
[ -n "$expected" ] || die "checksum manifest does not contain $asset"
actual="$(file_sha256 "$archive")"
[ "$actual" = "$expected" ] || die "SHA-256 mismatch for $asset"

mkdir -p "$RELEASES_DIR" "$INSTALL_DIR"
release_dir="$RELEASES_DIR/$version-$target"
if [ ! -d "$release_dir" ]; then
  stage_dir="$RELEASES_DIR/.staging.$version-$target.$$"
  [ ! -e "$stage_dir" ] || die "staging path already exists: $stage_dir"
  mkdir "$stage_dir"
  stage_created=1
  tar -xzf "$archive" -C "$stage_dir"
  validate_package "$stage_dir" "$version"
  mv "$stage_dir" "$release_dir"
  stage_created=0
else
  validate_package "$release_dir" "$version"
fi

replace_symlink "$CURRENT_LINK" "$release_dir"
replace_symlink "$BIN_PATH" "$CURRENT_LINK/bin/dcode"

if command -v xattr >/dev/null 2>&1; then
  xattr -dr com.apple.quarantine "$release_dir" 2>/dev/null || true
fi

"$BIN_PATH" --version >/dev/null
printf 'Installed dcode %s to %s\n' "$version" "$BIN_PATH"
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) printf 'Add %s to PATH to run dcode.\n' "$INSTALL_DIR" ;;
esac
