#!/usr/bin/env python3
"""Exercise a synthetic download on an unshown Hyprland workspace; touch only the probe window."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]


def hypr(*args):
    return subprocess.check_output(["hyprctl", *args], text=True)


def main():
    if not os.environ.get("WAYLAND_DISPLAY") or not os.environ.get("HYPRLAND_INSTANCE_SIGNATURE"):
        raise SystemExit("Run this compositor regression in a Hyprland Wayland session")
    subprocess.run(["cargo", "build", "--manifest-path", "native/Cargo.toml", "--locked",
                    "--example", "verify_hidden_download"], cwd=ROOT, check=True)
    with tempfile.TemporaryDirectory(prefix="ts4-hidden-download-") as directory:
        signals = Path(directory)
        log = signals / "wayland.log"
        workspace = "name:ts4-verification-" + uuid.uuid4().hex[:12]
        with log.open("w") as out:
            process = subprocess.Popen([
                str(ROOT / "native/target/debug/examples/verify_hidden_download"), str(signals),
            ], cwd=ROOT, env=dict(os.environ, WAYLAND_DEBUG="1"), stdout=out, stderr=out)
            window = None
            moved = False
            try:
                deadline = time.monotonic() + 15
                while not signals.joinpath("ready.json").exists() or window is None:
                    assert process.poll() is None, log.read_text()[-2000:]
                    assert time.monotonic() < deadline, "probe window did not become ready"
                    window = next((w for w in json.loads(hypr("clients", "-j")) if w["pid"] == process.pid), None)
                    time.sleep(0.05)
                assert not window["xwayland"], "regression must use native Wayland"
                selector = "address:" + window["address"]
                original = str(window["workspace"]["id"])
                hidden_at = time.monotonic()
                protocol_start = log.stat().st_size
                hypr("dispatch", "movetoworkspacesilent", workspace + "," + selector)
                moved = True
                time.sleep(1)
                fixture = json.loads(signals.joinpath("ready.json").read_text())
                Path(fixture["downloads"]).joinpath("fixture.package").write_bytes(b"synthetic browser package")
                deadline = time.monotonic() + 8
                while not signals.joinpath("complete").exists() and time.monotonic() < deadline:
                    assert process.poll() is None, log.read_text()[-2000:]
                    time.sleep(0.05)
                assert signals.joinpath("complete").exists(), "download did not import and enable while the Wayland window was hidden"
                assert Path(fixture["game"]).joinpath("fixture.package").read_bytes() == b"synthetic browser package"
                while time.monotonic() - hidden_at < 8:
                    assert process.poll() is None, log.read_text()[-2000:]
                    time.sleep(0.1)
                hidden_protocol = log.read_bytes()[protocol_start:].decode(errors="replace")
                pongs = [line for line in hidden_protocol.splitlines() if "xdg_wm_base#" in line and ".pong(" in line]
                assert pongs, "hidden window did not answer compositor pings"
                print(f"PASS: hidden native Wayland window installed and enabled the verified download; original preserved; {len(pongs)} compositor ping replies over eight seconds")
            finally:
                signals.joinpath("stop").touch()
                try:
                    if moved and process.poll() is None:
                        hypr("dispatch", "movetoworkspacesilent", original + ",address:" + window["address"])
                finally:
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        process.terminate()
                        process.wait(timeout=5)
                    Path("/tmp/ts4-hidden-download-wayland.log").write_text(log.read_text())
            assert process.returncode == 0, log.read_text()[-2000:]


if __name__ == "__main__":
    main()
