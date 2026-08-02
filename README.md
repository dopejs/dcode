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

The installer verifies the release archive against `dcode_SHA256SUMS`, installs the complete runtime package under `${CODEX_HOME:-~/.codex}/packages/standalone`, and exposes `dcode` through `~/.local/bin` by default. Override the command directory with `DCODE_INSTALL_DIR`; select an exact version with `DCODE_RELEASE=0.1.0`.

Supported release targets:

- macOS: Apple Silicon and Intel
- Linux glibc: arm64 and x86_64
- Windows: x86_64

Run `dcode update` to reinstall the newest GitHub Release through the same verified path.

## First login

Start `dcode`, then run `/login` in the TUI and enter a DeepSeek API key. The key is stored through DCode's configured credential backend. The DeepSeek balance appears in the status line after authentication.

## Publish a release

The workspace starts at version `0.1.0`. For later releases, update `workspace.package.version` in `codex-rs/Cargo.toml` and refresh the lockfile, then push a matching tag:

```sh
git tag -a dcode-v0.1.0 -m "DCode 0.1.0"
git push origin dcode-v0.1.0
```

The `dcode-release` GitHub Actions workflow builds all supported targets, assembles the canonical package layout (CLI, code-mode host, ripgrep, and sandbox helpers), publishes SHA-256 checksums and both installers, then creates the GitHub Release. macOS artifacts are ad-hoc signed but not Apple-notarized.

To build packages without creating a release, open **Actions → dcode-release → Run workflow**. The workflow uploads `dcode-release-<version>` containing every platform archive, both installers, and the checksum manifest. A matching `dcode-v*` tag runs the same build and additionally publishes that bundle as a GitHub Release.

## Development

Rust sources live under `codex-rs`. See [installing and building](./docs/install.md) and [contributing](./docs/contributing.md) for the inherited Codex development workflow.

This fork retains upstream Codex components and is licensed under the [Apache-2.0 License](LICENSE).
