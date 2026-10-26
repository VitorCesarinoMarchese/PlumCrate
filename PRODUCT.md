# Product

<!-- impeccable:product-schema 1 -->

## Platform

Linux native desktop, built with Fastframe and egui. This is not an iOS or Android interface. The desktop UI in `native/` uses the Rust domain library in `core/`.

## Users

The Sims 4 players managing their local mods and custom content.

## Product purpose

Browse a local mod library, inspect photos and files, and manage installed state without losing original files or metadata.

## Capabilities and constraints

Preserve all existing native management features: folder/ZIP import with new mods enabled by default in the selected game, direct user-initiated enable/disable with locking and ownership/collision checks, external migration, rename, explicit source attachment/removal, trash/restore, settings transfer, search and photo previews. CurseForge browsing supports provider search, content types, scoped categories, game versions, sorting, pagination, summaries and screenshots, using a key saved in local Settings. Project details also offer browser-assisted installation. Download in browser explicitly approves the real provider release and source for the selected game, with inline disclosure and no extra popup. The official /download/{fileId} page handles the download; after verifying its size and published checksums, the app imports and enables it without another click. Originals stay in place, and existing game files are never overwritten. Managed mods with explicitly attached CurseForge projects also support manual update checks, in-app notices, and opening the provider page for manual download. Browser imports save the verified installed-release baseline with the initial consent. Other imports require confirmation of the installed provider release before comparison. Users replace older local files themselves and record the new release afterward. Update checks use saved local credentials and do not download, install, or replace files. Product scope includes in-app mod downloads and updates initiated by the user. Source candidates must come from real provider data and require user confirmation. Permanent deletion is allowed only for app-managed mods already in Trash, after reviewing the exact entries, acknowledging that deletion cannot be undone, and confirming. Revalidate ownership and reviewed contents before deletion. Never delete unmanaged files or unrelated system trash.

Project inspection includes author descriptions, safe links, images and file-specific dependencies. Each dependency or resolved author-linked mod requires its own Download in browser action. Inspection saves no sources and approves no downloads. Missing or partial evidence stays visible. Loading placeholders reserve content geometry without animation.

Task progress, issues and recent messages live in header Activity, replacing the permanent bottom status bar. Its nonmodal panel has an explicit X close control; closing it never cancels work. Browser import cancellation and scoped retries retain their existing backend and approvals. Routine in-app notices expire after five seconds; errors take priority and require explicit dismissal. Both remain available in session history, which is held in memory and does not survive restart. History targets 30 messages without evicting undismissed errors. Reading routine notices in Activity prevents repeat toasts; repeated notices are distinct and unchanged errors are deduplicated. Confirmation/review modals suppress Activity and toasts. No operating-system notifications are added. Filesystem safety and source-attachment confirmation remain unchanged.

## Brand commitments

The user chose the Plumbob concept from five directions and approved implementation. Use a clean Sims-inspired interface with short, nontechnical labels, recognizable icons and personality. Preserve the Sims 4 Mod Manager name. The blue light rail, light default and green state accents share one identity with the approved charcoal dark theme: neutral backgrounds, raised cards, blue primary actions and restrained green state accents. The sidebar omits the slogan. Discover keeps filters behind an initially collapsed Filters button at every size, with the active filter count and current sort visible. Discover hides its heading, search, filters, inline consent disclosure and pagination while scrolling down results, restoring them on upward input or inside the quiet buffer near the current page end. Wheel, scrollbar and keyboard intent control visibility; idle layout and animation changes cannot reverse it. The quiet buffer near the page end keeps the toolbar visible during small downward movements. It applies when maximum results offset minus current offset is at most the current rendered toolbar height plus 96 logical points. Including the rendered height accounts for space freed by collapse, so collapse cannot manufacture a new bottom hit. While inside the buffer, the measured distance from the bottom stays anchored through viewport resize and toolbar reveal. Upward input releases the anchor before scrolling. Downward input can hide the toolbar again only after scrolling above the buffer. The global header and navigation stay visible. Search focus, expanded filters and provider popups keep the toolbar visible; Ctrl+F restores search immediately. Top, non-scrollable, loading, error and new-navigation states show the toolbar. Results scroll independently at every size; below 600 points of window height, expanded filters have their own bounded scroll area so Clear filters stays reachable. Shared library and Discover search keeps keyboard focus and clear controls visible; library filtering stays live and provider searches require Enter or Search. Photo cards remain the default library view with a compact list switch. Existing saved appearance preferences and custom themes remain intact.

## Evidence on hand

The current native implementation is in native/src. The migration completion report records filesystem behavior and validation. Existing user mod previews are content, not bundled artwork. Missing photos must show No Preview; never invent mod metadata or source candidates.

Header Activity evidence: `native/examples/verify_activity.rs` reproduces nine fixture-only captures in `.impeccable/review/header-activity`: `idle-dark`, `waiting-header`, `waiting-open`, `waiting-small`, `waiting-light`, `error-toast`, `error-open`, `success-toast` and `success-expired`. It starts no download and changes no user mods. The independent finish reviewer returned ship with no material fixes and all nine captures valid. `native/tests/activity_ui.rs` covers header/no-footer layout, wide/compact task cancellation, closing without cancellation and persistent error dismissal/history. Activity model tests cover expiration, deduplication, error priority and retention. All eight required repository validation commands passed for header Activity. Earlier review and validation results remain historical.

## Product principles

- Keep the app local-first.
- Keep management actions explicit and recoverable.
- Prioritize fast library browsing and photo recognition.
- Preserve complete file and source inspection.

Direct browser download evidence: fixture captures are in `.impeccable/review/direct-browser-download`. Independent review returned ship with no material fixes. All eight required repository validation commands and the controlled browser-dispatch check passed. No live mod download was used for validation.

Discover toolbar auto-hide evidence uses fixture-only states 10–12 in `native/examples/verify_discovery.rs` and captures in `.impeccable/review/discovery-autohide`. All eight required validation commands passed; independent review returned ship with no material fixes.

Bottom-buffer regression evidence: the extended `native/tests/discovery_ui.rs::reaching_bottom_anchors_results_through_the_entire_toolbar_reveal` checks page end, up 60, then down 20. Before the fix, the small downward movement transiently reduced toolbar height to 152.125 instead of 220; after the fix, every frame keeps the full toolbar height at 1200×800, 1870×1006 and 640×480. Up 500 then down 20 verifies hiding resumes outside the buffer. All 27 targeted discovery UI tests passed. Fixture state 15 in `native/examples/verify_discovery.rs` produces `browse-bottom-buffer.png` in `.impeccable/review/discovery-bottom-buffer`. All eight required validation commands passed. Independent review returned ship with no material fixes across four valid captures; this covers the scoped buffer regression, not a replay of the full original video.

Earlier bottom-anchor regression evidence: `native/tests/discovery_ui.rs::reaching_bottom_anchors_results_through_the_entire_toolbar_reveal` reaches the end through direct scrollbar state without wheel momentum. Before the fix it failed with offset 2645 against maximum 2704.875; after the fix it passes at 1200×800, 1870×1006 and 640×480, checking a steady last card throughout reveal and anchor release on upward input. All 27 discovery UI tests passed. Fixture captures are in `.impeccable/review/discovery-bottom-anchor`; `native/examples/verify_discovery.rs` state 14 produces `browse-bottom-anchor.png` from direct end-of-scroll state without wheel momentum and asserts the anchor. All eight required validation commands passed. Independent review returned ship with no material fixes across eight valid captures; the regression verifies the specific bottom-reveal defect, not a replay of the full user recording.
