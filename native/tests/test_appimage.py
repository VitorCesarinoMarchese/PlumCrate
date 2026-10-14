"""Verify the AppImage launcher runs a relocated release bundle."""
import importlib.util
import hashlib
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


NATIVE = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("appimage", NATIVE / "appimage.py")
appimage = importlib.util.module_from_spec(spec)
spec.loader.exec_module(appimage)


class AppImageTests(unittest.TestCase):
    def test_publishes_checksum_for_the_exact_appimage_filename(self):
        with tempfile.TemporaryDirectory() as directory:
            image = Path(directory) / "ts4-mod-manager-native-0.0.1-alpha.2-x86_64.AppImage"
            image.write_bytes(b"AppImage release fixture")
            checksum = appimage.write_checksum(image)
            self.assertEqual(checksum, image.with_name(image.name + ".sha256"))
            self.assertEqual(checksum.read_text(),
                             f"{hashlib.sha256(image.read_bytes()).hexdigest()}  {image.name}\n")

    def test_relocated_launcher_preserves_arguments_and_working_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bundle = root / "bundle"
            binary = bundle / "bin/ts4-mod-manager-native"
            binary.parent.mkdir(parents=True)
            binary.write_text('#!/bin/sh\nprintf "%s\\n" "$PWD" "$@"\n')
            binary.chmod(0o755)
            desktop = bundle / "share/applications/ts4-mod-manager-native.desktop"
            desktop.parent.mkdir(parents=True)
            desktop.write_text("[Desktop Entry]\nType=Application\nExec=ts4-mod-manager-native\n")
            icon = bundle / "share/icons/hicolor/256x256/apps/ts4-mod-manager-native.png"
            icon.parent.mkdir(parents=True)
            icon.write_bytes(b"fixture icon")
            license_file = bundle / "share/licenses/ts4-mod-manager-native/LICENSE"
            license_file.parent.mkdir(parents=True)
            license_file.write_text("fixture license")
            directory = root / "AppDir"
            appimage.create_appdir(bundle, directory)
            relocated = root / "relocated path with spaces"
            shutil.move(directory, relocated)
            output = subprocess.check_output(
                [str(relocated / "AppRun"), "--root", "game folder", "--inspect"],
                cwd=root, text=True,
            )
            self.assertEqual(output.splitlines(), [str(root), "--root", "game folder", "--inspect"])
            self.assertEqual((relocated / "ts4-mod-manager-native.desktop").read_bytes(), desktop.read_bytes())
            self.assertEqual((relocated / "ts4-mod-manager-native.png").read_bytes(), icon.read_bytes())
            self.assertEqual((relocated / "usr/share/licenses/ts4-mod-manager-native/LICENSE").read_bytes(), license_file.read_bytes())


if __name__ == "__main__":
    unittest.main()
