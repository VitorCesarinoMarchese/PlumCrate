#!/usr/bin/env python3
"""Capture the native library and Settings using isolated synthetic data."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
from verify import preview_png, snapshot, assert_preview_pixels


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path(__file__).parent / "target/release/ts4-mod-manager-native")
    parser.add_argument("--capture-dir", type=Path, required=True)
    args = parser.parse_args()
    binary = args.binary.resolve()
    args.capture_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ts4-navigation-") as directory:
        fixture = Path(directory)
        game, managed, home = [fixture / name for name in ["The Sims 4", "managed", "home"]]
        (game / "Mods").mkdir(parents=True)
        for index, name in enumerate(["Café Kitchen Collection", "Evening Essentials", "Everyday Sneakers", "Summer Linen Collection", "Weekend Wardrobe", "Winter Knitwear"]):
            mod_id = f"fixture-{index}"
            mod = managed / "mods" / mod_id
            (mod / "files").mkdir(parents=True)
            (mod / "files/content.package").write_bytes(b"synthetic navigation fixture")
            meta = {"version": 1, "createdBy": "sims4-mod-manager", "modId": mod_id, "name": name, "displayName": name, "slug": None, "files": ["content.package"], "source": "local"}
            if index == 0:
                (mod / "preview.png").write_bytes(preview_png())
                meta["localPreviewPath"] = str(mod / "preview.png")
            (mod / "meta.json").write_text(json.dumps(meta))
        base = ["--home", str(home), "--root", str(game), "--managed-root", str(managed)]
        before = snapshot(fixture)
        captures = [
            ("navigation-library-light.png", []),
            ("navigation-library-dark.png", ["--dark"]),
            ("navigation-library-narrow.png", ["--size", "680x760"]),
            ("navigation-library-minimum.png", ["--size", "640x480"]),
            ("navigation-library-short.png", ["--size", "1200x480"]),
            ("navigation-library-wide.png", ["--size", "1885x1000", "--dark"]),
            ("navigation-settings-light.png", ["--settings"]),
            ("navigation-settings-dark.png", ["--settings", "--dark"]),
            ("navigation-settings-narrow.png", ["--settings", "--size", "680x760"]),
        ]
        for name, flags in captures:
            path = args.capture_dir.resolve() / name
            subprocess.run([str(binary), *base, *flags, "--screenshot", str(path)], check=True, timeout=60)
            assert path.read_bytes().startswith(b"\x89PNG\r\n\x1a\n")
            if "library" in name:
                assert_preview_pixels(path)
        assert snapshot(fixture) == before, "Captures modified their inputs"
        settings_path = home / ".config/ts4-mod-manager/settings.json"
        settings_path.parent.mkdir(parents=True)
        settings = {"version": 1, "theme": "dark", "gameRoots": [], "activeCustomTheme": "Plum", "customThemes": [{"name": "Plum",
            "light": {"accent": "#64387A", "background": "#F8F3FA", "surface": "#FFFFFF", "text": "#251B2C", "mutedText": "#67566F", "border": "#DACDE1"},
            "dark": {"accent": "#B18BD0", "background": "#19151F", "surface": "#251E30", "text": "#F6F0FF", "mutedText": "#C7B8D8", "border": "#53445F"}}]}
        theme_captures = [
            ("navigation-theme-editor.png", "dark", []),
            ("navigation-theme-narrow.png", "dark", ["--size", "680x760"]),
            ("navigation-theme-light.png", "light", []),
            ("navigation-theme-light-narrow.png", "light", ["--size", "680x760"]),
        ]
        for name, mode, flags in theme_captures:
            settings["theme"] = mode
            settings_path.write_text(json.dumps(settings))
            before = snapshot(fixture)
            path = args.capture_dir.resolve() / name
            subprocess.run([str(binary), *base, "--settings", *flags, "--screenshot", str(path)], check=True, timeout=60)
            assert path.read_bytes().startswith(b"\x89PNG\r\n\x1a\n")
            assert snapshot(fixture) == before, "Custom-theme captures modified their inputs"
        for mode in ["light", "dark"]:
            settings["theme"] = mode
            settings_path.write_text(json.dumps(settings))
            before = snapshot(fixture)
            path = args.capture_dir.resolve() / f"navigation-custom-library-{mode}.png"
            subprocess.run([str(binary), *base, "--screenshot", str(path)], check=True, timeout=60)
            assert_preview_pixels(path)
            assert snapshot(fixture) == before, "Custom-theme library captures modified their inputs"
        print(json.dumps({"captures": len(captures) + len(theme_captures) + 2, "inputsUnchanged": True, "fixturePathsOnly": True}))


if __name__ == "__main__":
    main()
