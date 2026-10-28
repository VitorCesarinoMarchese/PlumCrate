"""Keep the public PlumCrate identity separate from existing ownership and data IDs."""
from pathlib import Path
import tomllib
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]


class BrandingTests(unittest.TestCase):
    def test_packages_and_documentation_name_plumcrate(self):
        native = tomllib.loads((ROOT / "native/Cargo.toml").read_text())
        core = tomllib.loads((ROOT / "core/Cargo.toml").read_text())
        self.assertEqual(native["package"]["name"], "plumcrate")
        self.assertEqual(core["package"]["name"], "plumcrate-core")
        self.assertIn("# PlumCrate\n", (ROOT / "README.md").read_text())
        self.assertIn("PlumCrate", "".join(ET.parse(ROOT / "native/assets/logo.svg").getroot().itertext()))

    def test_rebrand_keeps_existing_library_ownership_and_settings(self):
        self.assertIn('.local/share/sims4-mod-manager', (ROOT / "native/src/cli.rs").read_text())
        for file in ["native/src/main.rs", "native/src/controls.rs"]:
            self.assertIn('.config/ts4-mod-manager/settings.json', (ROOT / file).read_text())
        storage = (ROOT / "core/src/managed_storage.rs").read_text()
        self.assertIn('sims4-mod-manager', storage)
        self.assertIn('ts4-mod-manager-writer-', (ROOT / "core/src/operation.rs").read_text())

    def test_release_launcher_and_application_updates_use_the_same_identity(self):
        for file in ["native/package.py", "native/appimage.py", "native/src/app_update.rs"]:
            content = (ROOT / file).read_text()
            self.assertIn("plumcrate", content)
            self.assertNotIn("ts4-mod-manager-native", content)
        self.assertIn('Name=PlumCrate', (ROOT / "native/package.py").read_text())
        self.assertIn('VitorCesarinoMarchese/PlumCrate', (ROOT / "native/src/app_update.rs").read_text())


if __name__ == "__main__":
    unittest.main()
