#!/usr/bin/env python3
"""Reject accidental DCode changes outside the documented upstream seams."""

from pathlib import Path
import subprocess
import sys


REPO_ROOT = Path(__file__).resolve().parents[1]
ALLOWED_PREFIXES = (
    ".github/workflows/dcode-",
    "codex-rs/dcode-deepseek/",
    "codex-rs/dcode-product/",
    "scripts/dcode-",
    "scripts/install-dcode",
    "scripts/install/test_install_dcode.py",
    "scripts/check-downstream-boundary.py",
    "scripts/sync-upstream.sh",
)
TOUCHPOINTS_FILE = REPO_ROOT / ".github" / "dcode-upstream-touchpoints.txt"


def main() -> int:
    base = sys.argv[1] if len(sys.argv) > 1 else "upstream/main"
    touchpoints = {
        line.strip()
        for line in TOUCHPOINTS_FILE.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.startswith("#")
    }
    output = subprocess.check_output(
        ["git", "diff", "--name-only", f"{base}...HEAD"],
        cwd=REPO_ROOT,
        text=True,
        encoding="utf-8",
    )
    changed = [line for line in output.splitlines() if line]
    unexpected = [
        path
        for path in changed
        if path not in touchpoints
        and not any(path.startswith(prefix) for prefix in ALLOWED_PREFIXES)
    ]
    if unexpected:
        print("Unexpected files outside the DCode overlay boundary:", file=sys.stderr)
        for path in unexpected:
            print(f"  {path}", file=sys.stderr)
        return 1
    print(f"DCode boundary check passed ({len(changed)} changed files).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
