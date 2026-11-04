#!/usr/bin/env python3
"""Exercise an owned browser download through installation, Trash, and window closure."""
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    with tempfile.TemporaryDirectory(prefix="plumcrate-browser-proof-", dir=ROOT / "native" / "target") as directory:
        Path(directory, "plumcrate-browser-proof").touch()
        env = dict(os.environ, XDG_DATA_HOME=directory)
        subprocess.run([
            "cargo", "test", "--manifest-path", "native/Cargo.toml", "--locked",
            "--lib", "real_browser_download_installs_cleans_up_and_closes_window", "--",
            "--ignored", "--nocapture", "--test-threads=1",
        ], cwd=ROOT, env=env, check=True, timeout=300)
    print("PASS: real browser downloaded, installed, trashed the fixture and closed its window")


if __name__ == "__main__":
    main()
