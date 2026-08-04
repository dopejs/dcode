#!/usr/bin/env python3
"""Bump the DCode Rust workspace version for a release."""

import argparse
from pathlib import Path

from codex_package.version import VERSION_BUMPS
from codex_package.version import read_workspace_version_from
from codex_package.version import update_dcode_snapshot_versions
from codex_package.version import update_workspace_version


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bump", choices=VERSION_BUMPS, required=True)
    parser.add_argument(
        "--cargo-toml",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "codex-rs" / "Cargo.toml",
    )
    parser.add_argument(
        "--snapshot-root",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "codex-rs" / "tui" / "src",
    )
    args = parser.parse_args()
    current_version = read_workspace_version_from(args.cargo_toml)
    next_version = update_workspace_version(args.cargo_toml, args.bump)
    update_dcode_snapshot_versions(args.snapshot_root, current_version, next_version)
    print(next_version)


if __name__ == "__main__":
    main()
