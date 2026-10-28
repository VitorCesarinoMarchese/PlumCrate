#!/usr/bin/env python3
"""Build a Linux bundle and execute its binary against an isolated fixture."""
import argparse
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    args = parser.parse_args()
    native = Path(__file__).resolve().parent
    with tempfile.TemporaryDirectory(prefix="ts4-package-") as temporary:
        root = Path(temporary)
        subprocess.run([
            "python3", str(native / "package.py"), "--binary", str(args.binary.resolve()),
            "--output", str(root / "artifacts"),
        ], check=True)
        archives = list((root / "artifacts").glob("*.tar.gz"))
        assert len(archives) == 1
        with tarfile.open(archives[0]) as archive:
            archive.extractall(root / "unpacked", filter="data")
        bundle, = (root / "unpacked").iterdir()
        executable = bundle / "bin/plumcrate"
        assert executable.stat().st_mode & 0o111
        desktop = (bundle / "share/applications/plumcrate.desktop").read_text()
        assert "Exec=plumcrate" in desktop
        assert "Icon=plumcrate" in desktop
        assert (bundle / "share/icons/hicolor/256x256/apps/plumcrate.png").is_file()
        licenses = bundle / "share/licenses/plumcrate"
        assert (licenses / "Inter-LICENSE.txt").is_file()
        assert (licenses / "Lucide-LICENSE.txt").is_file()
        assert (licenses / "LICENSE").read_bytes() == (native.parent / "LICENSE").read_bytes()
        dependencies = json.loads((licenses / "dependencies.json").read_text())
        assert any(package["name"] == "eframe" for package in dependencies)
        game = root / "fixture/The Sims 4"
        game.joinpath("Mods").mkdir(parents=True)
        package = game / "Mods/example.package"
        package.write_bytes(b"original package")
        output = subprocess.check_output([
            str(executable), "--inspect", "--root", str(game),
            "--managed-root", str(root / "absent-managed"),
        ], text=True)
        assert json.loads(output)["total"] == 1
        assert package.read_bytes() == b"original package"
        assert not root.joinpath("absent-managed").exists()
        workflows = json.loads(subprocess.check_output(
            [str(executable), "--verify-workflows"], text=True, timeout=120,
        ))
        assert workflows["passed"] and workflows["fixturePathsOnly"]
        assert workflows["mods"] == 3
        print("Packaged executable, desktop assets, licenses, catalog and management workflows verified")


if __name__ == "__main__":
    main()
