# AppImage updates

The AppImage checks this repository's GitHub releases on startup and every
24 hours while open. In **Settings → App updates**, clear **Automatically check
for app updates** and save settings to turn off these requests. **Check for app
updates** runs a check on demand. No CurseForge API key is needed.

When a compatible release is available, select **App update available** in the
header or **Review app update** in Settings. Read the release notes, select
**Download app update**, and wait for verification. Select **Restart to update**
when ready. Finish mod operations and browser downloads, close other reviews,
and save or discard settings before restarting.

Downloads stay in a temporary file beside the installed AppImage until you
approve the restart. The app verifies the published size, SHA-256, AppImage
format, architecture, and reported version. It checks HTTPS download and
redirect destinations against GitHub release hosts. Checksums come from
GitHub's release asset digest or the release's `.sha256` asset. They detect
corrupt downloads; they are not a separate publisher signature.

Installation preserves the original filename and executable permissions.
Mods and settings stay in their existing folders. The previous AppImage is
kept in a `.ts4-previous-*` folder beside the installed copy as
`previous.AppImage`. These backups are retained. The current app closes only
after the updated window acknowledges startup. A failed launch or a startup
timeout restores the previous AppImage. If a file changes during restart,
the app preserves it and reports the backup location.

Automatic checks consider the 100 most recent releases. Alpha builds can
update to newer prereleases or stable releases; stable builds accept stable
releases. Drafts, older versions, and assets for other architectures are skipped.
An empty release list produces no update notice. Network and verification
failures leave the installed app unchanged and allow another attempt.

Self-updates require a regular, writable Linux AppImage in a writable folder.
Symlink launches and other installation formats show a manual download link.
The restarted AppImage uses extraction mode, so updating does not require FUSE.
Command-line inspection, workflow verification, version probes, and screenshot
captures do not check for updates.

AppImages released before this feature require one manual upgrade to a build
that includes it. No GitHub release is published by a local build or update check.

## Publish an update

1. Set `native/Cargo.toml` to the release version, such as `0.0.1-alpha.2`.
2. Run the full validation and baseline packaging steps in
   [Build and verify an AppImage](appimage.md).
3. Create a GitHub release in `VitorCesarinoMarchese/PlumCrate` with the
   matching version tag, such as `v0.0.1-alpha.2`.
4. Upload both packaging outputs to that release:

   ```text
   plumcrate-0.0.1-alpha.2-x86_64.AppImage
   plumcrate-0.0.1-alpha.2-x86_64.AppImage.sha256
   ```

The filename must match the manifest version and architecture exactly. The
packager writes the checksum file from the finished AppImage. Upload both files
before publishing the release. Use the prerelease flag for alpha releases.

## Verify a packaged update locally

The ignored native test reads an existing AppImage and exercises its version
probe, staged installation, preserved backup, and rollback inside a new
temporary fixture. The input AppImage is left unchanged:

```sh
TS4_TEST_APPIMAGE=/absolute/path/to/plumcrate-0.0.1-alpha.1-x86_64.AppImage \
  cargo test --manifest-path native/Cargo.toml --locked --lib \
  packaged_appimage_can_be_verified_installed_and_restored -- --ignored
```

This check runs without a graphical session. In a Linux desktop session, verify
the restarted window's startup acknowledgement and screenshot with:

```sh
TS4_TEST_APPIMAGE=/absolute/path/to/plumcrate-0.0.1-alpha.1-x86_64.AppImage \
  cargo test --manifest-path native/Cargo.toml --locked --lib \
  packaged_appimage_restart_acknowledges_a_rendered_window -- --ignored
```

The check, download, and restart flow follows
[Spotifast's updater](https://github.com/crmne/spotifast/blob/main/src/updates.rs).
Its shared Fastframe updater installs portable archives rather than AppImages,
so AppImage installation lives in `native/src/app_update.rs` here.
