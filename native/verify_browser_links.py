#!/usr/bin/env python3
"""Verify real desktop URL dispatch without opening a website or downloading mods."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
URL = "https://www.curseforge.com/sims4/mods/synthetic-fixture/download/100"


def main():
    subprocess.run([
        "cargo", "build", "--manifest-path", "native/Cargo.toml", "--locked",
        "--example", "verify_browser_links",
    ], cwd=ROOT, check=True)
    with tempfile.TemporaryDirectory(prefix="ts4-browser-dispatch-") as directory:
        fixture = Path(directory)
        browser = fixture / "controlled-browser"
        receipt = fixture / "receipt"
        browser.write_text('''#!/usr/bin/env python3
import os
from pathlib import Path
import sys
Path(os.environ["TS4_BROWSER_RECEIPT"]).write_text(sys.argv[1])
''')
        browser.chmod(0o700)
        env = dict(os.environ, BROWSER=str(browser), TS4_BROWSER_RECEIPT=str(receipt), TS4_VERIFY_BROWSER_LINKS="1")
        subprocess.run([
            str(ROOT / "native/target/debug/examples/verify_browser_links"), str(receipt),
        ], cwd=ROOT, env=env, check=True, timeout=30)
        assert receipt.read_text() == URL
        print("PASS: exact release URL dispatched to an external browser process")
        verify_stalled_resolver(fixture, browser, receipt)
        verify_desktop_fallback(fixture, receipt)


def verify_stalled_resolver(fixture, browser, receipt):
    receipt.unlink()
    started = fixture / "resolver-started"
    release = fixture / "resolver-release"
    resolver = fixture / "xdg-settings"
    resolver.write_text('''#!/usr/bin/env python3
import os
from pathlib import Path
import time
fixture = Path(os.environ["TS4_BROWSER_RECEIPT"]).parent
(fixture / "resolver-started").touch()
while not (fixture / "resolver-release").exists():
    time.sleep(0.02)
print("controlled-browser.desktop")
''')
    resolver.chmod(0o700)
    applications = fixture / "applications"
    applications.mkdir()
    (applications / "controlled-browser.desktop").write_text(
        f"[Desktop Entry]\nType=Application\nExec={browser} %u\nTerminal=false\n")
    env = dict(os.environ, TS4_BROWSER_RECEIPT=str(receipt), TS4_VERIFY_BROWSER_LINKS="1",
               PATH=str(fixture) + os.pathsep + os.environ["PATH"],
               XDG_DATA_HOME=str(fixture), XDG_DATA_DIRS=str(fixture))
    env.pop("BROWSER", None)
    process = subprocess.Popen([
        str(ROOT / "native/target/debug/examples/verify_browser_links"), str(receipt),
    ], cwd=ROOT, env=env)
    try:
        deadline = time.monotonic() + 10
        while not started.exists():
            assert process.poll() is None, "desktop probe exited before resolver started"
            assert time.monotonic() < deadline, "desktop resolver was never called"
            time.sleep(0.02)
        time.sleep(0.6)
        # A covered window may not repaint; inspect the blocked syscall on its actual event thread.
        main_wait = (Path("/proc") / str(process.pid) / "wchan").read_text().strip()
        assert "pipe" not in main_wait and main_wait != "do_wait", (
            f"desktop event thread waits for the blocked browser resolver: {main_wait}")
        assert not receipt.exists(), "resolver must remain blocked during responsiveness check"
        print(f"PASS: blocked browser lookup is off the desktop event thread ({main_wait})")
    finally:
        release.touch()
        process.wait(timeout=20)
    assert process.returncode == 0
    assert receipt.read_text() == URL


def verify_desktop_fallback(fixture, receipt):
    receipt.unlink()
    (fixture / "python3").symlink_to(sys.executable)
    (fixture / "xdg-settings").write_text("#!/bin/sh\necho missing-browser.desktop\n")
    opener = fixture / "xdg-open"
    opener.write_text("#!/bin/sh\nexit 1\n")
    opener.chmod(0o700)
    gio = fixture / "gio"
    gio.write_text('''#!/usr/bin/env python3
import os
from pathlib import Path
import sys
assert sys.argv[1] == "open"
assert len(sys.argv) == 3
Path(os.environ["TS4_BROWSER_RECEIPT"]).write_text(sys.argv[2])
''')
    gio.chmod(0o700)
    env = dict(os.environ, BROWSER="missing-browser", PATH=str(fixture),
               XDG_CURRENT_DESKTOP="Hyprland", DESKTOP_SESSION="hyprland",
               XDG_DATA_HOME=str(fixture), XDG_DATA_DIRS=str(fixture),
               TS4_BROWSER_RECEIPT=str(receipt), TS4_VERIFY_BROWSER_LINKS="1")
    subprocess.run([
        str(ROOT / "native/target/debug/examples/verify_browser_links"), str(receipt),
    ], cwd=ROOT, env=env, check=True, timeout=30)
    assert receipt.read_text() == URL
    print("PASS: failed browser discovery and xdg-open fall back to gio with the exact URL")


if __name__ == "__main__":
    main()
