#!/usr/bin/env python3
"""Convert the existing Linux release bundle into an AppImage."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile


def create_appdir(bundle, appdir):
    shutil.copytree(bundle / "bin", appdir / "usr/bin")
    shutil.copytree(bundle / "share", appdir / "usr/share")
    (appdir / "ts4-mod-manager-native.desktop").symlink_to(
        "usr/share/applications/ts4-mod-manager-native.desktop")
    (appdir / "ts4-mod-manager-native.png").symlink_to(
        "usr/share/icons/hicolor/256x256/apps/ts4-mod-manager-native.png")
    launcher = appdir / "AppRun"
    launcher.write_text('#!/bin/sh\nset -eu\n'
                        'appdir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\n'
                        'exec "$appdir/usr/bin/ts4-mod-manager-native" "$@"\n')
    launcher.chmod(0o755)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--appimagetool", type=Path, required=True,
                        help="Executable AppRun from an extracted, trusted appimagetool")
    parser.add_argument("--output", type=Path, default=Path(__file__).parent / "target/packages")
    args = parser.parse_args()
    native = Path(__file__).resolve().parent
    args.output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ts4-appimage-") as directory:
        staging = Path(directory)
        subprocess.run(["python3", str(native / "package.py"), "--binary", str(args.binary.resolve()),
                        "--output", str(staging / "bundles")], check=True)
        archive, = (staging / "bundles").glob("*.tar.gz")
        with tarfile.open(archive) as release:
            release.extractall(staging / "unpacked", filter="data")
        bundle, = (staging / "unpacked").iterdir()
        appdir = staging / "AppDir"
        create_appdir(bundle, appdir)
        destination = args.output.resolve() / (archive.name.removesuffix(".tar.gz").replace("-linux-", "-") + ".AppImage")
        env = os.environ.copy()
        env["ARCH"] = "x86_64"
        subprocess.run([str(args.appimagetool.resolve()), "--no-appstream",
                        "--mksquashfs-opt", "-processors", "--mksquashfs-opt", "1",
                        str(appdir), str(staging / "release.AppImage")], env=env, check=True)
        shutil.copy2(staging / "release.AppImage", destination)
        print(destination)


if __name__ == "__main__":
    main()
