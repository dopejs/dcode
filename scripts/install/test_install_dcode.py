#!/usr/bin/env python3

import hashlib
import os
from pathlib import Path
import platform
import subprocess
import tarfile
import tempfile
import unittest


INSTALL_SCRIPT = Path(__file__).parents[1] / "install-dcode.sh"
VERSION = "1.2.3"


class InstallDcodeShTest(unittest.TestCase):
    def test_installs_verified_package_and_managed_command(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            target = current_target()
            create_release(root, target)

            result = run_installer(root)

            self.assertEqual(result.returncode, 0, result.stderr)
            install_bin = root / "install-bin" / "dcode"
            current = root / "codex-home" / "packages" / "dcode-standalone" / "current"
            self.assertEqual(os.readlink(install_bin), str(current / "bin" / "dcode"))
            self.assertEqual(
                subprocess.check_output(
                    [install_bin, "--version"], text=True, encoding="utf-8"
                ).strip(),
                f"dcode-cli {VERSION}",
            )

            repeated = run_installer(root)

            self.assertEqual(repeated.returncode, 0, repeated.stderr)
            self.assertEqual(os.readlink(install_bin), str(current / "bin" / "dcode"))

    def test_does_not_reuse_codex_standalone_current_link(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            create_release(root, current_target())
            codex_release = (
                root / "codex-home" / "packages" / "standalone" / "releases" / "9.9.9"
            )
            codex_release.mkdir(parents=True)
            codex_current = codex_release.parents[1] / "current"
            codex_current.symlink_to(codex_release)

            result = run_installer(root)

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(os.readlink(codex_current), str(codex_release))
            self.assertEqual(
                subprocess.check_output(
                    [root / "install-bin" / "dcode", "--version"],
                    text=True,
                    encoding="utf-8",
                ).strip(),
                f"dcode-cli {VERSION}",
            )

    def test_refuses_to_overwrite_unmanaged_command(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            create_release(root, current_target())
            install_bin = root / "install-bin"
            install_bin.mkdir()
            unmanaged = install_bin / "dcode"
            unmanaged.write_text("user-owned\n", encoding="utf-8")

            result = run_installer(root)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("refusing to overwrite non-symlink path", result.stderr)
            self.assertEqual(unmanaged.read_text(encoding="utf-8"), "user-owned\n")


def current_target() -> str:
    machine = platform.machine().lower()
    arch = "aarch64" if machine in {"arm64", "aarch64"} else "x86_64"
    if platform.system() == "Darwin":
        return f"{arch}-apple-darwin"
    if platform.system() == "Linux":
        return f"{arch}-unknown-linux-gnu"
    raise unittest.SkipTest("the POSIX installer supports macOS and Linux")


def create_release(root: Path, target: str) -> None:
    package = root / "package"
    write_executable(
        package / "bin" / "dcode",
        f"#!/bin/sh\nprintf '%s\\n' 'dcode-cli {VERSION}'\n",
    )
    write_executable(package / "bin" / "codex-code-mode-host", "#!/bin/sh\nexit 0\n")
    write_executable(package / "codex-path" / "rg", "#!/bin/sh\nexit 0\n")
    (package / "codex-resources").mkdir()
    (package / "codex-package.json").write_text("{}\n", encoding="utf-8")

    release_dir = root / "releases" / f"dcode-v{VERSION}"
    release_dir.mkdir(parents=True)
    asset = f"dcode-package-{target}.tar.gz"
    archive = release_dir / asset
    with tarfile.open(archive, "w:gz") as bundle:
        for child in package.rglob("*"):
            bundle.add(child, arcname=child.relative_to(package), recursive=False)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    (release_dir / "dcode_SHA256SUMS").write_text(
        f"{digest}  {asset}\n", encoding="utf-8"
    )


def write_executable(path: Path, contents: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents, encoding="utf-8")
    path.chmod(0o755)


def run_installer(root: Path) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env.update(
        {
            "CODEX_HOME": str(root / "codex-home"),
            "DCODE_INSTALL_DIR": str(root / "install-bin"),
            "DCODE_RELEASE": VERSION,
            "DCODE_RELEASE_BASE_URL": (root / "releases").as_uri(),
            "HOME": str(root / "home"),
        }
    )
    return subprocess.run(
        ["sh", INSTALL_SCRIPT],
        env=env,
        text=True,
        encoding="utf-8",
        capture_output=True,
        check=False,
    )


if __name__ == "__main__":
    unittest.main()
