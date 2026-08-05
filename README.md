# DCode

DCode is a personal DeepSeek-focused fork of Codex CLI. It ships the `dcode` command, defaults to DeepSeek V4 Flash, supports API-key login, DeepSeek balance/model APIs, and an optional external vision model for image inputs.

## Install

macOS and Linux:

```sh
curl -fsSL https://github.com/dopejs/dcode/releases/latest/download/install-dcode.sh | sh
```

Windows PowerShell (x86_64):

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/dopejs/dcode/releases/latest/download/install-dcode.ps1 | iex"
```

The installer verifies the release archive against `dcode_SHA256SUMS`, installs the complete runtime package under `${DCODE_HOME:-~/.dcode}/packages/standalone`, and exposes `dcode` through `~/.local/bin` by default. Override the command directory with `DCODE_INSTALL_DIR`; select an exact version with `DCODE_RELEASE=0.2.0`.

Supported release targets:

- macOS: Apple Silicon and Intel
- Linux glibc: arm64 and x86_64
- Windows: x86_64

Run `dcode update` to reinstall the newest GitHub Release through the same verified path.

## First login

Start `dcode`, then run `/login` in the TUI and enter a DeepSeek API key. The key is stored through DCode's configured credential backend. The DeepSeek balance appears in the status line after authentication.

## Publish a release

Open **Actions → dcode-version-release → Run workflow** on the default branch and choose `patch`, `minor`, or `major`. The workflow updates DCode's downstream release version without changing upstream crate versions, commits the change, creates a matching `dcode-v*` tag, and starts the multi-platform release build. The tag build publishes the completed bundle as a GitHub Release.

The version commit and tag are pushed atomically. If dispatching the build fails afterward, rerun **dcode-release** manually and select the newly created tag.

The equivalent manual process is to update `DCODE_VERSION` in `codex-rs/dcode-product/src/lib.rs` and push a matching tag:

```sh
git tag -a dcode-v0.2.0 -m "DCode 0.2.0"
git push origin dcode-v0.2.0
```

The `dcode-release` GitHub Actions workflow builds all supported targets, assembles the canonical package layout (DCode launcher, sibling Codex runtime, code-mode host, ripgrep, and sandbox helpers), publishes SHA-256 checksums and both installers, then creates the GitHub Release. macOS artifacts are ad-hoc signed but not Apple-notarized.

To build packages without creating a release, open **Actions → dcode-release → Run workflow**. The workflow uploads `dcode-release-<version>` containing every platform archive, both installers, and the checksum manifest. A matching `dcode-v*` tag runs the same build and additionally publishes that bundle as a GitHub Release.

## Development

Rust sources live under `codex-rs`. See [the downstream architecture and sync guide](./DOWNSTREAM.md), [installing and building](./docs/install.md) and [contributing](./docs/contributing.md) for the inherited Codex development workflow.

This fork retains upstream Codex components and is licensed under the [Apache-2.0 License](LICENSE).
