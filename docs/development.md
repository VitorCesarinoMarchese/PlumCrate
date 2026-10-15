# Development and release packaging

Run commands in this guide from the repository root. For app controls, see the [native app reference](native-app.md).

The first alpha is labeled 0.01 in the README. Both crates and release archives use the Cargo version `0.0.1-alpha.1`. The project uses the [MIT License](../LICENSE), which is included in Linux release archives alongside dependency licenses.

## Run the app

Requirements: Rust 1.98 or newer, Cargo, and a graphical Linux session. Python 3.11 or newer is needed for verification and packaging.

Install native graphics build dependencies on Ubuntu or Debian:

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libxkbcommon-dev libwayland-dev libgl1-mesa-dev
```

Run from the repository root:

```bash
cargo run --manifest-path native/Cargo.toml --locked
```

Open a specific Sims 4 folder containing `Mods`:

```bash
cargo run --manifest-path native/Cargo.toml --locked -- --root '/path/to/The Sims 4'
```

Use `--settings` to open Settings directly. The [native app guide](native-app.md) describes all controls and CLI options. The app supports Linux; other desktop platforms have not been verified.

Capture the production review popups with synthetic data in light, dark and small-window layouts:

```bash
cargo run --manifest-path native/Cargo.toml --locked --example verify_popups -- native/target/popup-captures
```

This helper renders the app's actual controller and popup code. It creates temporary fixtures, captures ten states, and checks that no operation was approved and no settings were saved. It needs a graphical Linux session.

## Local storage and safety

Native file and folder selection uses `rfd` with the XDG desktop portal, as in [ZapFast's dependency configuration](https://github.com/crmne/zapfast/blob/main/Cargo.toml). A Linux desktop needs a working XDG portal and file-chooser backend for its native picker. `rfd` can fall back to Zenity when the portal is unavailable. The adapter in `native/src/file_picker.rs` returns paths on a channel; `Controls` retains each request's purpose and import identity, handles cancellation, and keeps existing confirmation and filesystem safety checks.

Managed files live in `~/.local/share/sims4-mod-manager`. Scanning does not create managed storage. Unreadable folders, file entries and damaged managed metadata report an incomplete scan rather than a successful partial catalog. Each managed mod keeps its metadata in `meta.json`.

Settings live in `~/.config/ts4-mod-manager/settings.json`. Saves use atomic replacement and restricted permissions. Custom theme colors persist locally and have a separate JSON export. Settings transfers omit the API key unless explicitly included.

Source lookup uses real CurseForge provider results and requires a locally configured API key. Requests run separately from local management, with a 20-second shared network budget and a 5-second request limit. Closing the app does not wait on read-only provider requests. Attachment always requires confirmation. Weak candidates retain their confidence warnings. Keep API keys outside Git, chat and per-mod metadata.

Enable/disable reviews exact paths before applying changes. Ownership checks protect replacement files and foreign symlinks. Trash keeps managed files recoverable; unmanaged files are not removed. Startup recovery preserves ambiguous or changed paths and reports issues. Approved operations finish before normal window shutdown.

## Validate changes

```bash
cargo test --manifest-path core/Cargo.toml --locked
cargo test --manifest-path native/Cargo.toml --locked
cargo clippy --manifest-path native/Cargo.toml --locked --all-targets -- -D warnings
cargo build --manifest-path native/Cargo.toml --release --locked
python3 -m unittest discover -s native/tests -p 'test_*.py'
python3 native/verify.py --binary native/target/release/ts4-mod-manager-native
native/target/release/ts4-mod-manager-native --verify-workflows
python3 native/verify_package.py --binary native/target/release/ts4-mod-manager-native
```

The catalog verifier checks 100, 1,000 and 10,000 synthetic mod groups and preserves their input files. The workflow verifier creates isolated fixtures and exercises reviewed management operations. The repository regression checks that the resolved build graph contains no Tauri or WebKit app dependencies.

Capture the actual native library and Settings from a graphical desktop:

```bash
python3 native/verify_navigation.py --capture-dir native/target/navigation-captures
```

The README screenshot comes from these isolated fixtures. To refresh it after inspecting the capture:

```bash
cp native/target/navigation-captures/navigation-library-light.png docs/images/library-light.png
```

## Package a Linux release

After building the release executable:

```bash
python3 native/package.py --binary native/target/release/ts4-mod-manager-native
```

CI builds release archives on Ubuntu 24.04 and rejects executables requiring glibc newer than 2.39. Use CI artifacts for distribution. Locally built executables inherit the host library requirements and may require a newer Linux installation. Other runtime graphics dependencies still need to be installed.

The archive appears under `native/target/packages/`. It contains the executable, desktop entry, icon and dependency licenses. The package verifier runs the extracted binary against isolated catalog and management fixtures.

## Repository layout

```text
core/                 Rust domain library and filesystem regression tests
native/               Fastframe desktop app, UI tests and verification tools
native/assets/        Desktop application icon
.github/workflows/    Core and native validation plus Linux release packaging
docs/                 App guides, development, design and historical evidence
docs/archive/         Retired app specifications and setup documents
```

Read the [management safety design](native-management-design.md) for ownership and recovery contracts, or the [native design system](native-design.md) for UI behavior and tokens. The [original investigation](fastframe-investigation.md) and [migration report](native-migration-report.md) describe historical revisions. Historical web source remains available in Git history, not in the active checkout.
