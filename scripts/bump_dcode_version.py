#!/usr/bin/env python3
"""Bump the DCode Rust workspace version for a release."""

import argparse
from pathlib import Path

from codex_package.version import VERSION_BUMPS, update_workspace_version


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bump", choices=VERSION_BUMPS, required=True)
    parser.add_argument(
        "--cargo-toml",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "codex-rs" / "Cargo.toml",
    )
    args = parser.parse_args()
    print(update_workspace_version(args.cargo_toml, args.bump))


if __name__ == "__main__":
    main()
