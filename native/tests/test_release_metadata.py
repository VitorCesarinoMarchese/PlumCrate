"""Verify the release metadata and the license shipped to users."""
from pathlib import Path
import subprocess
import tarfile
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ReleaseMetadata(unittest.TestCase):
    def test_alpha_version_and_project_license(self):
        for crate in ["core", "native"]:
            package = tomllib.loads((ROOT / crate / "Cargo.toml").read_text())["package"]
            self.assertEqual(package["version"], "0.0.1-alpha.1")
            self.assertEqual(package["license"], "MIT")

    def test_archive_contains_the_project_license(self):
        with tempfile.TemporaryDirectory(prefix="ts4-license-test-") as directory:
            root = Path(directory)
            binary = root / "fixture-binary"
            binary.write_text("#!/bin/sh\nexit 0\n")
            binary.chmod(0o755)
            subprocess.run([
                "python3", str(ROOT / "native/package.py"), "--binary", str(binary),
                "--output", str(root / "packages"),
            ], check=True, capture_output=True)
            archive_path, = (root / "packages").glob("*.tar.gz")
            with tarfile.open(archive_path) as archive:
                license_path = next((name for name in archive.getnames()
                                     if name.endswith("/share/licenses/plumcrate/LICENSE")), None)
                self.assertIsNotNone(license_path, "The release must include the project license")
                self.assertEqual(archive.extractfile(license_path).read(), (ROOT / "LICENSE").read_bytes())
