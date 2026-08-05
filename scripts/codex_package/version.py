"""Version discovery for Codex packages."""

import re
from pathlib import Path

from .targets import REPO_ROOT


WORKSPACE_VERSION_PATTERN = re.compile(r'^version\s*=\s*"([^"]+)"')
DCODE_VERSION_PATTERN = re.compile(
    r'^pub const DCODE_VERSION: &str = "([^"]+)";$', re.MULTILINE
)
STABLE_SEMVER_PATTERN = re.compile(
    r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$"
)
VERSION_BUMPS = ("major", "minor", "patch")


def read_workspace_version() -> str:
    cargo_toml = REPO_ROOT / "codex-rs" / "Cargo.toml"
    return read_workspace_version_from(cargo_toml)


def read_workspace_version_from(cargo_toml: Path) -> str:
    """Read `[workspace.package].version` from a Cargo workspace manifest."""
    in_workspace_package = False
    with open(cargo_toml, encoding="utf-8") as fh:
        for line in fh:
            stripped = line.strip()
            if stripped == "[workspace.package]":
                in_workspace_package = True
                continue

            if in_workspace_package and stripped.startswith("["):
                break

            if in_workspace_package:
                match = WORKSPACE_VERSION_PATTERN.match(stripped)
                if match is not None:
                    return match.group(1)

    raise RuntimeError(f"Could not find [workspace.package].version in {cargo_toml}")


def update_dcode_snapshot_versions(
    snapshot_root: Path, current_version: str, next_version: str
) -> list[Path]:
    """Update version-bearing DCode TUI snapshots for a release bump."""
    replacements = (
        (f"DCode (v{current_version})", f"DCode (v{next_version})"),
        (
            f"Update available! {current_version} ->",
            f"Update available! {next_version} ->",
        ),
    )
    updated: list[Path] = []
    for snapshot in sorted(snapshot_root.rglob("*.snap")):
        contents = snapshot.read_text(encoding="utf-8")
        revised = contents
        for old, new in replacements:
            revised = revised.replace(old, new)
        if revised != contents:
            snapshot.write_text(revised, encoding="utf-8")
            updated.append(snapshot)
    return updated


def next_workspace_version(current: str, bump: str) -> str:
    """Return the next stable semantic version for the requested bump."""
    match = STABLE_SEMVER_PATTERN.fullmatch(current)
    if match is None:
        raise ValueError(f"workspace version must be stable semver, got {current!r}")
    if bump not in VERSION_BUMPS:
        raise ValueError(f"unsupported version bump {bump!r}")

    major, minor, patch = (int(part) for part in match.groups())
    if bump == "major":
        return f"{major + 1}.0.0"
    if bump == "minor":
        return f"{major}.{minor + 1}.0"
    return f"{major}.{minor}.{patch + 1}"


def update_workspace_version(cargo_toml: Path, bump: str) -> str:
    """Bump `[workspace.package].version` and return the new version."""
    contents = cargo_toml.read_text(encoding="utf-8")
    lines = contents.splitlines(keepends=True)
    in_workspace_package = False
    version_index: int | None = None
    current: str | None = None

    for index, line in enumerate(lines):
        stripped = line.strip()
        if stripped == "[workspace.package]":
            in_workspace_package = True
            continue
        if in_workspace_package and stripped.startswith("["):
            break
        if in_workspace_package:
            match = WORKSPACE_VERSION_PATTERN.match(stripped)
            if match is not None:
                version_index = index
                current = match.group(1)
                break

    if version_index is None or current is None:
        raise RuntimeError(
            f"Could not find [workspace.package].version in {cargo_toml}"
        )

    next_version = next_workspace_version(current, bump)
    version_line = lines[version_index]
    newline = (
        "\r\n"
        if version_line.endswith("\r\n")
        else "\n"
        if version_line.endswith("\n")
        else ""
    )
    lines[version_index] = f'version = "{next_version}"{newline}'
    cargo_toml.write_text("".join(lines), encoding="utf-8", newline="")
    return next_version


def read_dcode_version_from(product_source: Path) -> str:
    """Read the downstream release version without coupling it to Cargo."""
    contents = product_source.read_text(encoding="utf-8")
    match = DCODE_VERSION_PATTERN.search(contents)
    if match is None:
        raise RuntimeError(f"Could not find DCODE_VERSION in {product_source}")
    return match.group(1)


def update_dcode_version(product_source: Path, bump: str) -> str:
    """Bump only the DCode product version and return the new version."""
    contents = product_source.read_text(encoding="utf-8")
    current = read_dcode_version_from(product_source)
    next_version = next_workspace_version(current, bump)
    revised, count = DCODE_VERSION_PATTERN.subn(
        f'pub const DCODE_VERSION: &str = "{next_version}";', contents
    )
    if count != 1:
        raise RuntimeError(
            f"Expected one DCODE_VERSION in {product_source}, got {count}"
        )
    product_source.write_text(revised, encoding="utf-8", newline="")
    return next_version
