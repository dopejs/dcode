from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from .version import next_workspace_version
from .version import read_workspace_version_from
from .version import update_dcode_snapshot_versions
from .version import update_workspace_version


class VersionTest(unittest.TestCase):
    def test_next_workspace_version(self) -> None:
        self.assertEqual(next_workspace_version("1.2.3", "patch"), "1.2.4")
        self.assertEqual(next_workspace_version("1.2.3", "minor"), "1.3.0")
        self.assertEqual(next_workspace_version("1.2.3", "major"), "2.0.0")

    def test_rejects_prerelease_version(self) -> None:
        with self.assertRaisesRegex(ValueError, "stable semver"):
            next_workspace_version("1.2.3-rc.1", "patch")

    def test_updates_only_workspace_package_version(self) -> None:
        with TemporaryDirectory() as directory:
            cargo_toml = Path(directory) / "Cargo.toml"
            cargo_toml.write_text(
                '[package]\nversion = "9.9.9"\n\n'
                '[workspace.package]\nversion = "0.1.0"\nedition = "2024"\n',
                encoding="utf-8",
            )

            version = update_workspace_version(cargo_toml, "minor")

            self.assertEqual(version, "0.2.0")
            self.assertEqual(
                cargo_toml.read_text(encoding="utf-8"),
                '[package]\nversion = "9.9.9"\n\n'
                '[workspace.package]\nversion = "0.2.0"\nedition = "2024"\n',
            )

    def test_reads_workspace_version_from_path(self) -> None:
        with TemporaryDirectory() as directory:
            cargo_toml = Path(directory) / "Cargo.toml"
            cargo_toml.write_text(
                '[package]\nversion = "9.9.9"\n\n'
                '[workspace.package]\nversion = "0.1.1"\n',
                encoding="utf-8",
            )

            self.assertEqual(read_workspace_version_from(cargo_toml), "0.1.1")

    def test_updates_only_version_bearing_dcode_snapshots(self) -> None:
        with TemporaryDirectory() as directory:
            snapshot_root = Path(directory)
            status_snapshot = snapshot_root / "status.snap"
            status_snapshot.write_text(
                "│ >_ DCode (v0.1.1) │\n│ Update available! 0.1.1 -> 9.9.9 │\n",
                encoding="utf-8",
            )
            unrelated_snapshot = snapshot_root / "unrelated.snap"
            unrelated_snapshot.write_text("model version 0.1.1\n", encoding="utf-8")

            updated = update_dcode_snapshot_versions(snapshot_root, "0.1.1", "0.1.2")

            self.assertEqual(updated, [status_snapshot])
            self.assertEqual(
                status_snapshot.read_text(encoding="utf-8"),
                "│ >_ DCode (v0.1.2) │\n│ Update available! 0.1.2 -> 9.9.9 │\n",
            )
            self.assertEqual(
                unrelated_snapshot.read_text(encoding="utf-8"),
                "model version 0.1.1\n",
            )


if __name__ == "__main__":
    unittest.main()
