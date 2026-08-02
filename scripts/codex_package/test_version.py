from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from .version import next_workspace_version, update_workspace_version


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


if __name__ == "__main__":
    unittest.main()
