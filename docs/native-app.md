# Native app reference

Run commands in this guide from the repository root. See the [development guide](development.md) for build requirements and full validation.

The Fastframe/egui UI uses the existing Rust core to detect game folders, scan mods, search names and filenames, and manage local mods. Photo cards are the default library view; the card/list switch offers compact rows. Cards, catalog rows and file lists render only visible rows. A replaceable reader handles scans. A separate FIFO worker retains approved management operations and reports their queued, running, and finished states.

Import folder is the primary action in the library toolbar, beside Import ZIP and Restore from trash. Settings in the header opens a full page. Select a managed mod for direct Enable/Disable controls in the details popup. More actions contains Enable/Disable, rename, source lookup/attachment/removal and trash. Right-click a photo card or list row to select that mod and open the same menu. External mods offer Manage external mod with a file review before migration. The compact bottom status shows local-library information, notices and errors; Activity expands operation history when present.

Import folder and Import ZIP open your desktop's file picker. ZIP selection filters for ZIP archives. After selection, review the chosen path and edit the suggested mod name before selecting Import. Canceling the picker does not import anything. A changed game folder or selection invalidates a pending import selection.

App reviews appear as centered popups over a dimmed library. Import, rename, source attachment, settings import and management confirmations share this layout. Long paths wrap, long reviews scroll, and the action row remains visible. Cancel, Escape or a click on the backdrop dismisses a review without applying it. Source candidates and trash entries also open in popups; reviewing an entry replaces that popup with its confirmation.

Choose folder and Change folder open the desktop folder picker for the Sims 4 folder containing Mods. You can also enter a game path manually. Settings import opens a JSON file picker and then shows the existing confirmation review. Settings export opens a save dialog suggesting `ts4-mod-manager-settings.json`; exports still refuse to overwrite an existing file.

The picker starts in Downloads when that folder exists, otherwise in your home directory, and remembers the last selected location during the session. Change folder starts in the current game folder. Only one desktop picker can be open at a time. Picker work runs separately from approved management operations and does not block the UI or normal shutdown.

## Browse CurseForge

**Browse mods** opens a separate Sims 4 CurseForge browser without requiring a game folder. It uses the API key saved in local Settings. Unsaved key edits do not affect requests. The **Open Settings** action appears when the key is missing or rejected.

With a saved key, the browser loads popular projects. Search submits through **Search** or Enter; Ctrl+F focuses the browser search field.

**Browse by** selects a provider content type: Mods, Create a Sim, Build / Buy, Rooms / Lots, Sims / Households, Pets, Save Files, Worlds, or Translations. **Category** becomes available for the selected type and shows searchable parent paths, such as Body / Hairstyles. Changing the content type clears the category. **Game Version** lists searchable numeric Sims 4 versions, newest first. These options come from CurseForge.

**Sort by** offers Relevancy, Popularity, Latest update, Date released, Total Downloads, and A-Z. Relevancy uses the provider's default ranking. Date released uses the public API's release-date sort; the website's Creation Date sort is not available through that API. **Clear filters** clears content type, category, and game version while retaining search text and sort.

Submitting a search or changing a filter or sort restarts at the first page. **Previous** and **Next** retain the submitted search, content type, category, version, and sort. Pages contain 24 results, within the API's 10,000-result limit. Editing a search without submitting it does not change page navigation. Below 600 logical points of window height, **Filters** folds the selectors and shows the active filter count. Expanded controls scroll with the results, so **Clear filters** remains reachable in short windows.

Project cards show provider names, authors, logos, summaries, categories, download counts, and modification dates. **View details** opens the full summary and provider screenshots inside the app. **Back to results** or Escape restores the current results. **Open on CurseForge** and **Copy link** appear only when the API supplies a usable HTTPS project URL. Broken provider images hide. Preview requests omit Referer.

Switching to **My mods** preserves local search and selection. Browsing does not install or update mods, write mod metadata, or attach sources. Use the existing import controls for files you already have.

Requests run in a separate read-only worker. Pending searches coalesce and late responses cannot replace newer results. The worker loads category and version options when they are missing or a prior attempt failed. Loading, no matches, missing or rejected credentials, rate limits, network errors, and unavailable providers have separate states. Each request uses the existing bounded CurseForge transport. Normal shutdown does not wait for provider I/O.

The ignored integration test `live_browse_filters_respect_provider_taxonomy` in `core/tests/curseforge_browse.rs` passed against the live provider. It checked all nine content types, combined Mods / Gameplay / 1.128.90 filters, filtered pagination, and real file-version evidence. Fixture captures in `.impeccable/review/curseforge-filters` cover both themes, the 1870-by-1006 user window, 680-by-760 and 640-by-480 windows, filter popups, and expanded filters in a short window. Credentials used for live tests stay local and untracked; the app still requires a key saved in Settings.

The capture example uses only synthetic API responses and local procedural images:

```sh
cargo run --manifest-path native/Cargo.toml --locked --example verify_discovery -- native/target/ui-discovery
```

Live searches need a valid locally saved key. The implementation was checked against fixtures because no key was available during development.

## Build and launch

Requires Rust 1.98 or later and native Linux graphics support. Fastframe is pinned to `bb79dbddef01e660f9cfc37ccd9dff1c299a8d47`. `Cargo.lock` pins the resolved dependencies, including egui/eframe 0.36.2. The native app uses published egui/winit rather than the optional Fastframe forks.

Run from the project root:

```bash
cargo run --manifest-path native/Cargo.toml --locked
```

Open a custom game folder, the directory containing `Mods`:

```bash
cargo run --manifest-path native/Cargo.toml --locked -- \
  --root '/path/to/The Sims 4'
```

## Browse the library

The window follows the desktop theme, with a light/dark switch that immediately applies and saves the chosen mode while retaining custom colors. Select Change folder to open the desktop folder picker. Detected game folders remain available above the library, and you can enter a path and select Open during manual folder entry. Search works over display names and filenames. The library shows large photo cards by default, and details show a larger preview. All, Installed and Stored navigation filters intersect the current name/filename search. Filter counts come from the actual catalog. Changing a filter does not mutate files. Switching between cards and list rows preserves search and selected identity. Saved local PNG/JPEG/WebP/GIF images and HTTP(S) URLs load in the background without a Referer header. Missing, broken, and unsupported previews show No Preview. Remote photos are cached under `~/.cache/ts4-mod-manager/photos`. They refresh after seven days, with valid older photos retained if refresh fails offline. The cache is limited to 128 MiB; individual photos are limited to 8 MiB, and downloads omit the Referer header. Local mod photos are read in place. Source attachment and removal require explicit confirmation. Select a card or row to inspect files and saved source metadata. Wide windows have a navigation rail. Below 1040 logical points, state tabs replace the rail. Selecting a card or row opens a centered details popup over the library at every width. Close, Escape, or clicking outside returns to browsing without clearing the selection. Management reviews temporarily replace the details popup and return to it after cancellation. Copy copies the full file list and confirms with Copied. Files show their basename above the containing folder; truncated names have full-path tooltips.

## Storage and command-line inspection

The default managed root is `~/.local/share/sims4-mod-manager`. Scanning does not create it. Override it with `--managed-root PATH`. Use `--help` for all options. Unreadable entries and damaged managed metadata report an incomplete scan. The UI retains the previous successful catalog.

Headless inspection uses the same scan and search model:

```bash
native/target/debug/ts4-mod-manager-native \
  --root '/path/to/The Sims 4' --inspect --query 'filename'
```

## Manage mods and sources

Enable and disable first show their exact paths and warnings. Confirmation revalidates the approved review. Migration shows the external file set. Trash accepts managed mods and requires confirmation. Restore lists recoverable entries and requires a destination review. An approved operation remains associated with its original root and entry after navigation. Closing the window waits for approved operations to finish.

Find source uses real CurseForge provider results and reports a missing API key. Candidates show confidence, reasons, and evidence. Low-confidence warnings stay visible in attachment review. Manual URLs carry no verified-match claim. The native app does not download or update mods.

Startup asks the core to recover pending operations and displays recovery issues. Native toggle and migration hold the shared writer guard across revalidation and mutation. A forced process termination can interrupt approved work; core recovery preserves changed or ambiguous user paths and may require attention before further mutations. [Twelve core process-crash scenarios](process-crash-verification.md) cover real SIGKILL and fresh-process recovery. Desktop-window termination, reboot, touchpad, monitor, and screen-reader checks still need separate evidence.

## Settings and custom themes

Settings replaces the central library. Back to library returns to browsing; on wide windows, selecting a library filter also leaves Settings. The page scrolls below its title and always-visible Save settings and Discard actions. Use `--settings` to open it directly. Light, Dark, System and saved custom-theme choices apply immediately in Settings; select Save settings to persist them. The header light/dark switch saves only the appearance mode immediately. Other edits remain drafts until Save settings. Leaving the page keeps drafts; Discard restores saved preferences, including both theme palettes, while keeping any header mode change.

Each custom theme has separate light and dark palettes. The header switch and the Light, Dark and System appearance choices use the matching palette. Use default colors explicitly returns to the built-in palette, retaining saved custom themes; select Save settings to persist that choice.

Create custom theme starts a named draft with both built-in palettes. Choose Light or Dark above the editor to edit that version's six #RRGGBB fields: Accent, Background, Surface, Text, Muted text and Border. Each valid field has a color picker. Switching modes preserves edits in both versions. Reset colors restores only the displayed version. Copy theme JSON copies both palettes. Import a theme accepts paired JSON or an older single-palette theme. Older themes keep their original colors in the version matching their background brightness and use built-in colors for the companion version. Save settings or the top Save settings validates both palettes, applies and persists the draft. Invalid colors, an empty name or a conflicting saved-theme name produce an error.

Settings save locally in `~/.config/ts4-mod-manager/settings.json` using atomic replacement and mode 0600 on Unix. Save settings persists the theme, local custom-theme collection and active custom-theme name, remembered game folders, selected folder, and optional API key. CLI overrides apply to that launch. Settings export requires an unused file path and omits the API key unless Include API key in export is checked. It also omits custom themes and their active name for compatibility with version-1 transfers from older releases; use the separate theme JSON copy/import controls to transfer custom colors. Settings import reads at most 1 MiB, validates and previews version 1 JSON before confirmation. It preserves the existing API key when omitted and the local custom-theme collection when the imported collection is empty or absent. Keys stay outside mod metadata and operation history.

## Verify and capture the app

Run tests and repeatable verification:

```bash
cargo test --manifest-path native/Cargo.toml --locked
cargo build --manifest-path native/Cargo.toml --locked
python native/verify.py
native/target/debug/ts4-mod-manager-native --verify-workflows
```

`--verify-workflows` creates its own temporary fixtures and uses the same native controller and management worker as the UI. It verifies folder and ZIP imports, reviewed enable and disable in two instances, migration, rename, manual source changes, trash and restore, missing-key errors, refreshed catalog data, and byte preservation. It prints a JSON report and ignores user paths for its fixtures.

`verify.py` creates temporary catalogs of 100, 1,000, and 10,000 mod groups, exercises the actual binary, checks Unicode/filename search and invalid paths, and verifies that input bytes and paths remain unchanged. It reports scan/search timings. For native screenshots, run this from a graphical desktop:

```bash
python native/verify.py --sizes 100 --capture-dir native/target/captures
```

This captures real light, dark, and narrow windows. To capture the current navigation and custom-theme editor on a graphical desktop:

```bash
python3 native/verify_navigation.py --capture-dir native/target/navigation-captures
```

`verify_navigation.py` defaults to the release binary; pass `--binary PATH` to use another build. It creates isolated synthetic game, managed and home directories and produces eleven PNGs: four library views, three Settings views and four custom-theme editor views covering light and dark versions at wide and narrow sizes. It checks preview pixels in library captures and verifies that fixture inputs remain unchanged. Screenshot mode fixes the window size and quits after writing the PNG. Normal launches remain resizable. For release measurements, build with `cargo build --manifest-path native/Cargo.toml --release --locked` and pass `--binary native/target/release/ts4-mod-manager-native` to the script.

## Historical performance measurements

The pre-redesign migration build scanned 10,000 synthetic groups in 53 ms. On 1,000-group fixtures, three launches used median process-tree PSS of 60 MiB for native and 208 MiB for the compared Tauri release. Median window mapping was 153 ms and 173 ms. These historical fixture measurements exclude input-to-paint latency and GPU frame times; they were not repeated for the library redesign. See the [completion report](native-migration-report.md) for raw evidence, method and remaining desktop checks.

Measure native window mapping and process memory on a running Hyprland desktop without opening real game data:

```bash
python3 native/benchmark.py --native native/target/release/ts4-mod-manager-native \
  --mods 1000 --runs 3 \
  --output native/target/benchmark.json
```

## Package the app

Build a Linux release archive with a desktop entry, icon and dependency license inventory:

```bash
python3 native/package.py
python3 native/verify_package.py --binary native/target/release/ts4-mod-manager-native
```

Archives are written under `native/target/packages`. Pass `--binary PATH` to package an existing build or `--output PATH` to choose another destination. Packaging does not install anything. The archive's README explains local installation. The executable uses the host OpenGL driver and X11 or Wayland libraries. The verifier extracts the archive and runs its actual executable against an isolated catalog. CI builds on Ubuntu 24.04, verifies a glibc 2.39 maximum requirement, runs native tests, clippy, release fixture checks and package verification, then uploads the Linux archive. Local builds inherit the host library requirements. Fetch the complete locked dependency graph with `cargo fetch --manifest-path native/Cargo.toml --locked` before offline package metadata verification on a fresh checkout.

## Implementation and safety notes

Unreadable scan entries and damaged mod metadata produce an incomplete-scan error; the UI retains the previous successful catalog. Recovery holdings are excluded from catalog entries. Provider lookups use a separate read-only worker, a shared 20-second network budget and 5-second request caps. DNS resolution cannot be interrupted by the HTTP library, so normal shutdown detaches that worker while continuing to wait for approved filesystem operations. Only one provider lookup can be queued through the UI at a time.

The scanner ownership fix and native controller use failing-before regression tests. Scanner tests prove managed mods stay separate when they share a folder with each other or external files, and relative managed links report installed state. Native tests cover stale success/error rejection, stable identities, same-instance refresh, Unicode/filename search, pending-request replacement, unchanged temporary inputs, CLI validation, and actual egui search interaction and virtualized rendering.

See [the investigation](fastframe-investigation.md) for the original bug inventory and [the chosen design](native-pilot-design.md) for the original controller rationale. See [native management design](native-management-design.md) for the current safety contracts. The investigation inventory describes the earlier revision.

Navigation, layout, folder and file icons are vendored from [Lucide](https://github.com/lucide-icons/lucide/tree/main/icons), with white strokes for Fastframe tinting. Their license is included in `native/licenses/Lucide-LICENSE.txt`.

Preview textures retain at most 32 image URIs and reset on Rescan or a game-folder change. The remote disk cache survives those resets and app restarts. Local relative paths resolve inside that instance’s Mods folder. Browser-only blob/asset URLs and data URIs are unsupported. Detail previews preserve aspect ratio with contain sizing; cards and list thumbnails use centered cover crops that account for the destination proportions. The summary scrolls independently so Files and Copy remain reachable in short windows.

The reviewed library design and tokens are recorded in [DESIGN.md](native-design.md) and [the native design manifest](../.impeccable/design.json). The [surface brief](../.impeccable/surfaces/native-library.md) records the approved direction. This is a Linux egui UI; the HTML/CSS design detector does not apply.

Navigation and Settings validation passed 46 native tests, native clippy and the release build. The earlier migration also validated the React/Tauri app before its retirement. Current validation uses the commands in the [development guide](development.md). Nine native navigation captures passed independent design review with a ship disposition. The 100, 1,000 and 10,000-group catalog checks preserved their input files; the packaged executable passed its 18-operation workflow check. These checks do not certify untested hardware or screen-reader behavior.

### Browsing and readability

Cards and List have visible labels. Ctrl+F focuses search; details have Previous/Next controls and Alt+Left/Right shortcuts that follow the current filter. Help (F1) explains imports, sources, recoverable removal, settings and keyboard controls. Escape goes back one level: an open menu closes first; Help and management reviews return to the underlying page or mod details; mod details return to the library. From Settings, Escape returns to the library even with a text field focused or background work running. Unsaved settings drafts, search and selection stay intact. At the library, Escape keeps the app open. Views without saved previews use smaller cards.

Custom palettes below 4.5:1 for their text, metadata or links display contrast warnings. The affected mode uses built-in colors until corrected, preserving the stored palette for editing. Reset colors restores the displayed version. Settings exports and source lookup use saved preferences.
