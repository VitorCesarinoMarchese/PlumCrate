# Build and verify an AppImage

Use an Ubuntu 22.04 amd64 root filesystem to compile against glibc 2.35.
The host needs `bwrap`, `nice`, `ionice`, Git, Python, and a Rust toolchain
that satisfies `native/Cargo.toml`. User namespaces must be enabled.

## Prepare the build environment

Download [Ubuntu Base 22.04.5](https://cdimage.ubuntu.com/ubuntu-base/releases/22.04/release/)
and verify the archive against its published SHA256SUMS before extracting it.
Keep the root filesystem under `native/target/appimage-baseline/rootfs`.
The amd64 archive used for this build has SHA256
`242cd8898b33ea806ef5f13b1076ed7c76f9f989d18384452f7166692438ff1a`.

Install the following packages inside that root filesystem, using a container
or a bubblewrap user namespace. Do not install them on the host for this build.

```text
build-essential pkg-config ca-certificates curl git python3 xvfb xauth
libgl1-mesa-dri libglx-mesa0 libegl1 libxkbcommon0 libxkbcommon-x11-0
libxcursor1 libxi6 libxrandr2 libxinerama1 libwayland-client0
libwayland-cursor0 libwayland-egl1 libdbus-1-3 fonts-dejavu-core
desktop-file-utils squashfs-tools file
```

When using bubblewrap, run package installation with
`apt-get -o APT::Sandbox::User=root`. A single-user namespace cannot create
the separate users required by system services. This environment needs the
D-Bus client library but does not need a system D-Bus daemon.

Verification requires Python 3.11 or newer. Ubuntu 22.04's default Python 3.10
cannot run the project's TOML metadata tests. Install a newer Python inside the
root filesystem, or extract a checksum-verified
[Python standalone build](https://github.com/astral-sh/python-build-standalone/releases)
under `rootfs/opt/python`. The verification script checks `/opt/python/bin`
before the system Python. This run uses Python 3.13.16 from release `20261009`.

Download a trusted [appimagetool release](https://github.com/AppImage/appimagetool/releases).
Verify its release checksum, then run `--appimage-extract` from its own directory.
Pass the extracted `squashfs-root/AppRun` to the verification script.
Keep tools outside the root filesystem, for example under
`native/target/appimage-baseline/tools`.

Populate the host Cargo caches with the locked dependencies before running the
offline build. The verification script mounts the registry and Git caches into
the namespace. It does not mount Cargo credentials or the host home directory.

## Run the checks in order

From the repository root, run:

```sh
python3 native/verify_baseline.py \
  --rootfs native/target/appimage-baseline/rootfs \
  --appimagetool native/target/appimage-baseline/tools/squashfs-root/AppRun
```

The script copies the working tree without ignored files or `.env` files, then
runs the core tests, desktop tests, Clippy, release build, Python tests, catalog
verification, management workflows, and archive verification. It packages the
AppImage only after those checks pass.

Finally, it runs catalog and management checks through the AppImage runtime and
captures desktop, dark, and narrow screenshots under Xvfb with software OpenGL.
The screenshot verifier checks that fixture preview pixels were painted and
that the fixture files remain unchanged. The runtime uses extraction mode, so
these checks do not require FUSE.

Cargo uses one job, tests use one thread, CPU niceness is 15, disk priority is
idle, and SquashFS compression uses one processor. The script stops at the
first failing check. Logs, build outputs, screenshots, and the AppImage remain
under `native/target/appimage-baseline/build`.

## Package an existing baseline build

Run this command inside the baseline environment:

```sh
python3 native/appimage.py \
  --binary /build/target/release/plumcrate \
  --appimagetool /tools/AppRun \
  --output /build/packages
```

The AppImage contains the existing release bundle, desktop entry, icon, and
dependency licenses. It uses the host graphics drivers and X11 or Wayland
libraries. `appimagetool` may download its AppImage runtime during packaging.
For a fully reproducible release, pin the tool and runtime versions.

Packaging also writes a `.AppImage.sha256` file beside the AppImage. Publish both
files in a GitHub release with a tag matching `native/Cargo.toml` to offer an
in-app update. See [AppImage updates](app-updates.md) for release requirements.

Automated checks cover the Ubuntu user-space baseline and software rendering.
Hardware graphics, portal file dialogs, Wayland integration, desktop installation,
and FUSE mounting still need testing in a desktop session before a public release.
