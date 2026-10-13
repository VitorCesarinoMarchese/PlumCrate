#!/usr/bin/env python3
"""Build and verify an AppImage in an Ubuntu 22.04 rootfs using bubblewrap.

The rootfs must have build-essential, pkg-config, Python, Git, CA certificates,
Xvfb, xauth, Mesa, X11/Wayland libraries, fonts, and desktop-file-utils installed.
The host supplies a Rust toolchain and Cargo's registry/git caches, not libraries.
"""
import argparse
import os
from pathlib import Path
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rootfs", type=Path, required=True)
    parser.add_argument("--appimagetool", type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    work = repo / "native/target/appimage-baseline"
    source = work / "source"
    source.mkdir(parents=True, exist_ok=True)
    # Copy the working tree, including this recipe, without exposing local keys or targets.
    files = subprocess.check_output(["git", "ls-files", "-z", "--cached", "--others",
                                     "--exclude-standard"], cwd=repo).decode().split("\0")
    for name in filter(None, files):
        if Path(name).name.startswith(".env"):
            continue
        destination = source / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(repo / name, destination)
    build = work / "build"
    build.mkdir(exist_ok=True)
    toolchain = Path(subprocess.check_output(["rustup", "which", "rustc"], text=True).strip()).parent.parent
    cargo = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    base = ["nice", "-n", "15", "ionice", "-c", "3", "bwrap", "--clearenv", "--unshare-user",
            "--uid", "0", "--gid", "0", "--bind", str(args.rootfs.resolve()), "/",
            "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp",
            "--ro-bind", "/etc/resolv.conf", "/etc/resolv.conf",
            "--ro-bind", str(source), "/workspace", "--bind", str(build), "/build",
            "--ro-bind", str(toolchain), "/opt/rust",
            "--ro-bind", str(args.appimagetool.resolve().parent), "/tools",
            "--bind", str(cargo / "registry"), "/cargo/registry",
            "--bind", str(cargo / "git"), "/cargo/git",
            "--setenv", "PATH", "/opt/python/bin:/opt/rust/bin:/usr/bin:/bin",
            "--setenv", "HOME", "/tmp/home", "--setenv", "CARGO_HOME", "/cargo",
            "--setenv", "CARGO_TARGET_DIR", "/build/target",
            "--setenv", "CARGO_BUILD_JOBS", "1", "--setenv", "RUST_TEST_THREADS", "1",
            "--setenv", "LIBGL_ALWAYS_SOFTWARE", "1", "--chdir", "/workspace"]
    binary = "/build/target/release/ts4-mod-manager-native"
    commands = [
        ["cargo", "test", "--manifest-path", "core/Cargo.toml", "--locked", "--offline"],
        ["cargo", "test", "--manifest-path", "native/Cargo.toml", "--locked", "--offline"],
        ["cargo", "clippy", "--manifest-path", "native/Cargo.toml", "--locked", "--offline", "--all-targets", "--", "-D", "warnings"],
        ["cargo", "build", "--manifest-path", "native/Cargo.toml", "--release", "--locked", "--offline"],
        ["python3", "-m", "unittest", "discover", "-s", "native/tests", "-p", "test_*.py"],
        ["python3", "native/verify.py", "--binary", binary],
        [binary, "--verify-workflows"],
        ["python3", "native/verify_package.py", "--binary", binary],
        ["python3", "native/appimage.py", "--binary", binary,
         "--appimagetool", "/tools/" + args.appimagetool.name, "--output", "/build/packages"],
        ["bash", "-ec", 'image=(/build/packages/*.AppImage); test "${#image[@]}" = 1; '
         'export APPIMAGE_EXTRACT_AND_RUN=1; '
         'python3 native/verify.py --binary "${image[0]}"; '
         '"${image[0]}" --verify-workflows; '
         'xvfb-run -a -s "-screen 0 1280x900x24" python3 native/verify.py '
         '--binary "${image[0]}" --sizes 100 --capture-dir /build/screenshots'],
    ]
    for index, command in enumerate(commands, 1):
        print(f"STEP {index}/{len(commands)}: {' '.join(command)}", flush=True)
        with (build / f"step-{index:02}.log").open("w") as log:
            result = subprocess.run(base + command, stdout=log, stderr=subprocess.STDOUT)
        if result.returncode:
            print((build / f"step-{index:02}.log").read_text()[-6000:])
            raise SystemExit(result.returncode)
        print(f"PASS {index}: {build / f'step-{index:02}.log'}", flush=True)
    print(f"Verified AppImage and screenshots: {build}")


if __name__ == "__main__":
    main()
