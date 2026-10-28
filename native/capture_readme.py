#!/usr/bin/env python3
"""Capture the native app with real cover art and isolated demonstration files."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
from urllib.request import Request, urlopen

from verify import snapshot

# Public project and cover URLs from explicitly attached CurseForge metadata.
COVERS = [
    ("Aurum - maxis match denim shorts", "AuSims", 1266137,
     "https://www.curseforge.com/sims4/create-a-sim/aurum-maxis-match-denim-shorts",
     "https://media.forgecdn.net/avatars/1274/361/638829051417443712.png"),
    ("Botanica Tattoo", "moonmoonsim", 739810,
     "https://www.curseforge.com/sims4/create-a-sim/botanica-tattoo",
     "https://media.forgecdn.net/avatars/678/867/638068403334197764.png"),
    ("GoldFish Little Black Dress LBD3", "GoldFishSims", 1135183,
     "https://www.curseforge.com/sims4/create-a-sim/goldfish-little-black-dress-lbd3",
     "https://media.forgecdn.net/avatars/1112/546/638664841782167451.png"),
    ("GoldFish Urban Elegance", "GoldFishSims", 1138449,
     "https://www.curseforge.com/sims4/create-a-sim/goldfish-urban-elegance",
     "https://media.forgecdn.net/avatars/1773/192/639126370020867232.png"),
    ("Aurum - Serana hairstyle", "AuSims", 1291154,
     "https://www.curseforge.com/sims4/create-a-sim/aurum-serana-hairstyle",
     "https://media.forgecdn.net/avatars/1324/263/638860931844527814.png"),
    ("MC Command Center", "deaderpool_mccc", 663350,
     "https://www.curseforge.com/sims4/mods/mc-command-center",
     "https://media.forgecdn.net/avatars/593/897/637967028550536035.png"),
    ("QICC - Sweater Weather Collection", "Quirky_Introvert", 803088,
     "https://www.curseforge.com/sims4/create-a-sim/qicc-sweater-weather-collection",
     "https://media.forgecdn.net/avatars/748/297/638087897421968953.jpeg"),
    ("WonderfulWhims", "TURBODRIVER", 670886,
     "https://www.curseforge.com/sims4/mods/wonderfulwhims",
     "https://media.forgecdn.net/avatars/602/578/637983317249528036.png"),
]


def cover_bytes(cover):
    with urlopen(Request(cover[4], headers={"User-Agent": "PlumCrate README capture"}), timeout=30) as response:
        data = response.read(8 * 1024 * 1024 + 1)
    if len(data) > 8 * 1024 * 1024 or not data.startswith((b"\x89PNG\r\n\x1a\n", b"\xff\xd8")):
        raise ValueError(f"Invalid cover image for {cover[0]}")
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path(__file__).parent / "target/release/plumcrate")
    parser.add_argument("--output", type=Path, default=Path(__file__).resolve().parent.parent / "docs/images")
    args = parser.parse_args()
    binary, output = args.binary.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    with ThreadPoolExecutor(max_workers=4) as pool:
        images = list(pool.map(cover_bytes, COVERS))
    with tempfile.TemporaryDirectory(prefix="ts4-readme-") as directory:
        fixture = Path(directory)
        game, managed, home = [fixture / name for name in ("The Sims 4", "managed", "home")]
        (game / "Mods").mkdir(parents=True)
        for index, ((title, author, project, source, cover), image) in enumerate(zip(COVERS, images)):
            mod_id = f"readme-{project}"
            mod = managed / "mods" / mod_id
            (mod / "files").mkdir(parents=True)
            filename = f"demo-{project}.package"
            package = mod / "files" / filename
            package.write_bytes(b"README demonstration file; not a playable mod")
            preview = mod / "cover"
            preview.write_bytes(image)
            metadata = {
                "version": 1, "createdBy": "sims4-mod-manager", "modId": mod_id,
                "name": title, "displayName": title, "files": [filename], "source": "local",
                "sourceUrl": source, "localPreviewPath": str(preview),
                "sourceAttachment": {"providerId": "curseforge", "projectId": project,
                    "sourceUrl": source, "title": title, "author": author,
                    "confidence": 100, "attachedBy": "user", "attachedAt": "2026-10-19T00:00:00Z"},
            }
            (mod / "meta.json").write_text(json.dumps(metadata))
            if index in (0, 3, 5, 6):
                (game / "Mods" / filename).symlink_to(package)
        base = [str(binary), "--home", str(home), "--root", str(game), "--managed-root", str(managed), "--size", "1440x1000"]
        before = snapshot(fixture)
        for name, flags in [("library-light.png", []), ("library-dark.png", ["--dark"]), ("settings.png", ["--settings"])]:
            path = output / name
            subprocess.run([*base, *flags, "--screenshot", str(path)], check=True, timeout=60)
            assert path.read_bytes().startswith(b"\x89PNG\r\n\x1a\n")
            assert snapshot(fixture) == before, "Screenshot changed demo inputs"
    print(json.dumps({"screenshots": 3, "realCovers": len(COVERS), "inputsUnchanged": True,
                      "coverSha256": [hashlib.sha256(image).hexdigest() for image in images]}))


if __name__ == "__main__":
    main()
