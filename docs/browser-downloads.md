# Browser-assisted mod installation

A user approves one CurseForge release, Downloads folder, and Sims 4 game folder. The initial review authorizes saving its source, copying the verified download into the managed library, and enabling it in that game. The app opens the official release page; the website or browser may still require a Download or save click. The app detects the completed file and finishes installation without another import or enable click. A saved local API key is required for provider metadata.

The [app guide](native-app.md#install-from-a-browser-download) describes **Browse mods**, **Download via website…**, and **Start browser download**. This path supports website downloads when API downloading is unavailable. It installs mods; it does not automatically replace older releases. [Update checks](mod-updates.md) retain their manual replacement workflow.

## File identity and installation

Project details show dependencies declared for the selected release file. The parent's selected file carries into its download review; switching the file in the review updates the displayed dependency relations. Required, optional, and tool relations offer a separate review after resolving the real provider project. Incompatible, embedded, included, and unknown relations offer inspection without a download action. Missing or partially readable data shows a warning rather than claiming there are no dependencies.

Each dependency or author-linked mod uses its own **Start browser download** consent and the same size, checksum, library, and game checks as the parent. Only one browser installation can run at a time. There is no automatic dependency queue or recursive resolution. Description links are not declared dependencies; an exact provider URL and slug match is required before a linked-mod review becomes available. [Project-details contracts](project-details.md) describe these read-only checks.

The provider supplies the exact project URL, real release identity, expected file length, and SHA1 or MD5 checksums. Eligible releases are stable, public ZIP, `.package`, or `.ts4script` files up to 2 GiB. Releases without usable size or checksum data are unavailable for automatic installation. The release navigation URL derives from the validated API project URL and real file ID. The saved source stays the exact project URL returned by the API.

The watcher runs for up to ten minutes in the approved folder. Its default comes from XDG Downloads configuration, falling back to `~/Downloads`; the review offers a desktop folder picker. It accepts stable regular files whose length and published checksums match. Browser temporary files and symbolic links are excluded. Filename equality is insufficient. Renamed downloads with a supported extension and already-completed matching files can be used after the initial confirmation.

The installer copies and verifies the same held file descriptor into private staging. It safely extracts ZIPs and keeps `.package` and `.ts4script` files intact. The original download remains unchanged. Source metadata and an installed-content digest are written to `meta.json` before publishing the managed mod. The source has `attachedBy: "user"`; the installed-release baseline supports subsequent update checks. API keys remain in local settings and never enter mod metadata.

Import, game links, and ownership records share one transaction. Existing files are never overwritten, and unmanaged files remain untouched. A collision fails safely. An intact managed copy of the same project and release is reused and enabled in the approved game rather than duplicated. Changed source or content on an existing copy requires review instead of silent reuse.

## Session ownership and cancellation

Native `browser_import::BrowserImport` owns the approved game, release, Downloads folder, session token, and publication permit. `Controls` renders the review and persistent status. `discovery` offers the project action. Core `browser_import` validates provider metadata, watches and verifies files, and performs the import-and-enable transaction.

The native `desktop_links` adapter takes browser URL requests from the UI and launches the desktop browser on a separate thread. A slow default-browser lookup is isolated from the window event thread. Clipboard commands stay with the desktop backend.

A separate Wayland rendering issue can block a stock runtime when another window covers it. The app pins Fastframe's matched egui/winit revisions to pause presentation safely while continuing background logic and compositor responses. The Hyprland regression installs and enables a synthetic download on an unshown workspace, preserving the original and checking ping replies during eight seconds out of view.

Provider reads and watching run outside the serialized filesystem queue. Only a finite verified installation enters that queue. A controller-owned session follows the existing ownership of saved settings, user confirmations, and management operations.

Cancel, game-folder changes, saved-key changes, and app closure revoke authorization before publication. Selection changes and ordinary rescans preserve the session. Once installation claims publication, it finishes or rolls back against the originally approved folders. The UI then reports that installation is finishing and disables cancellation. A blocked rollback retains the source and recovery journal until dependent links can be removed safely.

## Native UI evidence

The review preserves the existing charcoal and warm light surfaces, gold action, native typography, 12-point corners, and popup shadow. At 640 by 480, the game destination and approval/cancel row stay visible below the scrolling release details and Downloads-folder controls. Waiting status shows the watched folder and cancellation in the persistent bottom area. Tokens and bundled artwork are unchanged.

`native/examples/verify_browser_import.rs` captures production widgets with synthetic provider metadata. Its six captures in `.impeccable/review/browser-import-*.png` cover dark/light 1200-by-800 reviews, a 1871-by-1006 review, a 640-by-480 review, and wide/small waiting states. They download and install no mod files. Run the example with the capture directory as its argument, as shown in the app guide.

Independent finish review returned ship with no material fixes. Separate input tests prove one approval followed by automatic import and enabling. The small-window test proves the game destination and approval actions remain reachable. Core and native regression checks cover file identity, cancellation, collision handling and transaction recovery.
