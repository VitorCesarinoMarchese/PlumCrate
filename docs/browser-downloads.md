# Browser-assisted mod installation

**Download in browser** approves the selected CurseForge source and release for the selected game. An inline note explains that verified files install automatically and the source is saved. The app opens the official `/download/{fileId}` page without an extra confirmation popup; the browser may still require a save action. A saved local API key and selected Sims 4 folder are required.

Discover cards choose the latest eligible release. Project details retain a release selector. Projects already attached to a local mod show **View in library** on cards; details still allow an intentional release choice. Each dependency or resolved author-linked mod needs its own explicit download click. There is no dependency queue or recursive installation.

## File identity and installation

The provider supplies the canonical project URL, real release identity, expected length, and SHA1 or MD5 checksums. Eligible releases are stable public ZIP, `.package`, or `.ts4script` files up to 2 GiB. Files without usable length or checksums are unavailable for automatic installation. Navigation uses the validated project URL plus `/download/{fileId}`. Saved metadata retains the canonical API project URL.

The watcher runs for up to ten minutes. Its default folder comes from XDG Downloads configuration, falling back to `~/Downloads`. Change it in **Settings → Folders → Browser download folder**. This choice lasts for the session and cannot change during preparation or installation. An explicit folder change before retry preserves the approved game and release.

Only stable regular files with matching length and published checksums qualify. Temporary files and symbolic links are excluded. Renamed supported files and already-completed matching downloads can qualify after the explicit action. Filename equality alone is insufficient.

The installer copies and verifies the held file descriptor into private staging, safely extracts ZIPs, and preserves original downloads. Source metadata (`attachedBy: "user"`) and the verified installed-release baseline are saved in `meta.json`; API keys never enter mod metadata.

Import, game links, and ownership records share one transaction. Existing files are never overwritten and unmanaged files remain untouched. An intact managed copy of the same project and release is reused. Changed source or content requires review. This flow does not replace older releases automatically; [update checks](mod-updates.md) retain manual replacement.

## Session ownership and cancellation

`BrowserImport` owns the approved game, release, folder, session token and publication permit. `Controls` records an explicit pending action, synchronizes the game and saved key, then dispatches once when metadata is ready. Metadata readiness alone cannot start a download. Preparation is nonmodal; app restart is blocked while it is pending. Provider reads and watching run outside the serialized filesystem queue; only verified installation enters it.

Header **Activity** shows preparation, progress, errors, Cancel and scoped Retry. Closing Activity never cancels work. Cancel, game-folder or saved-key changes, and app closure revoke authorization before publication. Once publication starts, installation finishes or rolls back against the approved folders. Selection changes and ordinary rescans preserve the session.

The `desktop_links` adapter launches the browser on a separate thread. The pinned GUI runtime keeps background installation responsive when the window is covered. Existing collision, rollback and ownership checks remain in force.

## Native UI evidence

The approved charcoal/light palettes, blue actions and native typography are preserved. Fixture captures in `.impeccable/review/direct-browser-download/discovery` and `flow` cover labels, inline consent, nonmodal preparation and waiting at desktop and 640×480 sizes. They download no files. The independent reviewer returned ship with no material fixes.

Controller tests verify one explicit action, one browser dispatch, automatic verified installation, canonical source storage, cancellation and game-change invalidation. `native/verify_browser_links.py` checks the exact synthetic route and stalled-browser responsiveness without opening a website.

Direct-download validation: all eight AGENTS.md commands and `native/verify_browser_links.py` passed. Fixture tests cover dispatch and installation without downloading live mods.
