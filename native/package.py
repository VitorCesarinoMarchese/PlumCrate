#!/usr/bin/env python3
"""Create a Linux release bundle without installing anything on the host."""
import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tarfile
import tempfile


def main():
    native = Path(__file__).resolve().parent
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, help="Package an already-built executable")
    parser.add_argument("--output", type=Path, default=native / "target/packages")
    args = parser.parse_args()
    if platform.system() != "Linux":
        parser.error("Linux release bundles must be built on Linux")
    if args.binary is None:
        subprocess.run(["cargo", "build", "--manifest-path", str(native / "Cargo.toml"),
                        "--release", "--locked"], check=True)
    binary = (args.binary or native / "target/release/plumcrate").resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        parser.error("The executable does not exist or is not executable")
    metadata = json.loads(subprocess.check_output([
        "cargo", "metadata", "--manifest-path", str(native / "Cargo.toml"),
        "--locked", "--offline", "--format-version", "1",
    ], text=True))
    package = next(item for item in metadata["packages"] if item["name"] == "plumcrate")
    bundle_name = f"plumcrate-{package['version']}-linux-{platform.machine()}"
    args.output.mkdir(parents=True, exist_ok=True)
    destination = args.output / f"{bundle_name}.tar.gz"
    with tempfile.TemporaryDirectory(prefix=".ts4-package-", dir=args.output) as temporary:
        staging = Path(temporary)
        bundle = staging / bundle_name
        executable = bundle / "bin/plumcrate"
        executable.parent.mkdir(parents=True)
        shutil.copy2(binary, executable)
        desktop = bundle / "share/applications/plumcrate.desktop"
        desktop.parent.mkdir(parents=True)
        desktop.write_text("[Desktop Entry]\nType=Application\nName=PlumCrate\n"
                           "Comment=Manage your local Sims 4 mod collection\n"
                           "Exec=plumcrate\nIcon=plumcrate\n"
                           "Terminal=false\nCategories=Game;Utility;\nStartupNotify=true\n")
        icon = bundle / "share/icons/hicolor/256x256/apps/plumcrate.png"
        icon.parent.mkdir(parents=True)
        shutil.copy2(native / "assets/icon.png", icon)
        licenses = bundle / "share/licenses/plumcrate"
        shutil.copytree(native / "licenses", licenses)
        shutil.copy2(native.parent / "LICENSE", licenses / "LICENSE")
        inventory = []
        for dependency in sorted(metadata["packages"], key=lambda item: (item["name"], item["version"])):
            source = Path(dependency["manifest_path"]).parent
            license_files = [path for path in source.iterdir() if path.is_file()
                             and path.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE"))]
            relative_paths = []
            for license_file in license_files:
                target = licenses / "crates" / f"{dependency['name']}-{dependency['version']}" / license_file.name
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(license_file, target)
                relative_paths.append(str(target.relative_to(licenses)))
            inventory.append({"name": dependency["name"], "version": dependency["version"],
                              "license": dependency["license"], "files": relative_paths})
        (licenses / "dependencies.json").write_text(json.dumps(inventory, indent=2) + "\n")
        (bundle / "README.txt").write_text(
            "PlumCrate, native Linux build\n\n"
            "Run bin/plumcrate from a graphical Linux session.\n"
            "Requires the host OpenGL driver and X11 or Wayland client libraries.\n"
            "To install for your user, copy bin/ into ~/.local/bin/ and share/ into ~/.local/share/.\n"
            "Ensure ~/.local/bin is on PATH. This archive does not install automatically.\n"
            "Settings and managed files use your local application data directory.\n"
        )
        archive_path = staging / "bundle.tar.gz"
        with tarfile.open(archive_path, "w:gz") as archive:
            archive.add(bundle, arcname=bundle_name)
        os.replace(archive_path, destination)
    print(destination.resolve())


if __name__ == "__main__":
    main()
