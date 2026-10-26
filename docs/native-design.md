---
name: Sims 4 Mod Manager native
description: Plumbob, a simple Sims-inspired home for local mods.
colors:
  accent-light: "#1263CA"
  accent-dark: "#89B4FF"
  panel-light: "#EDF4F8"
  panel-dark: "#17191E"
  surface-light: "#FFFFFF"
  surface-dark: "#22252C"
  text-light: "#173452"
  text-dark: "#EDF0F5"
  muted-light: "#52687C"
  muted-dark: "#ACB4C0"
  border-light: "#D4E0E9"
  border-dark: "#3D444F"
  selection-fill-light: "#DFECCB"
  selection-fill-dark: "#293E32"
  selection-stroke-light: "#2E5710"
  selection-stroke-dark: "#A3D6AC"
  widget-fill-light: "#FFFFFF"
  widget-fill-dark: "#22252C"
  hover-light: "#E1EDF5"
  hover-dark: "#2D323C"
  rail: "#0863B9"
  pressed-dark: "#343B47"
  rail-dark: "#1C1F26"
  rail-selected-dark: "#2C3C55"
  rail-selected-text-dark: "#B8D2FF"
  primary-dark: "#3774D6"
  primary-text: "#FFFFFF"
  switch-on-dark: "#4E8451"
  switch-off-dark: "#57606E"
typography:
  headline:
    fontSize: "32pt"
  title:
    fontSize: "23pt"
  card-title:
    fontSize: "15pt"
  body:
    fontSize: "13pt"
  label:
    fontSize: "12pt"
rounded:
  widget: "9pt"
  search: "10pt"
  card: "12pt"
spacing:
  item-horizontal: "10pt"
  item-vertical: "8pt"
  card-gap: "16pt"
  panel: "24pt"
components:
  card-light:
    backgroundColor: "{colors.surface-light}"
    textColor: "{colors.text-light}"
    rounded: "{rounded.card}"
    height: "238pt"
  card-dark:
    backgroundColor: "{colors.surface-dark}"
    textColor: "{colors.text-dark}"
    rounded: "{rounded.card}"
    height: "238pt"
  button-primary-light:
    backgroundColor: "{colors.accent-light}"
    textColor: "{colors.surface-light}"
    rounded: "{rounded.widget}"
    padding: "10pt 14pt"
  button-primary-dark:
    backgroundColor: "{colors.primary-dark}"
    textColor: "{colors.primary-text}"
    rounded: "{rounded.widget}"
    padding: "10pt 14pt"
---

# Design System: Sims 4 Mod Manager native

## Overview

**Creative North Star: "Plumbob"**

The user chose Plumbob from five concepts and approved its implementation. A blue navigation rail, pale blue background, white cards and green installed-state accents give the app a Sims-inspired identity. Keep words short, use recognizable icons, and let actual mod photos carry the library's personality. The charcoal dark counterpart keeps the same hierarchy with raised neutral cards, blue primary actions and restrained green state accents.

This guide describes the Linux Fastframe/egui app. `native/src/settings.rs::ThemeColors::base` and `native/src/ui.rs::base_style` define the built-in colors. Rust widgets are authoritative; dimensions use egui logical points. Fresh settings default to light. Existing saved appearance choices and custom themes remain intact.

**Key Characteristics:**

- Blue light navigation and charcoal dark navigation with Your mods, Discover and Updates.
- Photo cards and compact list rows with direct On/Off switches.
- Short labels, original plumbob and compass icons, and recoverable management actions.

## Colors

### Primary

Blue identifies links and primary actions, including Add mods, Search and Downloads. The light rail keeps its blue fill; the dark rail uses charcoal with a subdued blue selected destination. Built-in primary actions use the light accent in light mode and the separate primary-dark token in dark mode, both with white text; menus return to the content theme. Active custom palettes also style the rail, its selected destination, Add mods, and mod switches. The custom surface and text colors style the rail, its selection uses the existing selection fill, and the accent styles the primary action and enabled switches. Action text and switch knobs start with the surface color and adjust for readability against their painted fills. Switching back to a built-in theme restores Plumbob colors.

### Secondary

Green selection fills and strokes identify selection and installed state. Use the selected-card outline for focus and selection, without recoloring the entire photo.

### Neutral

Pale blue panels and white cards form the light theme. Charcoal panels and raised neutral surfaces form the dark counterpart. Dark inactive widgets use the surface fill, hover and open states use hover-dark, and pressed states use pressed-dark. Enabled switches use switch-on-dark; disabled switches use switch-off-dark. Main text, explicit muted text and borders come from the paired tokens above. Hover fills provide immediate feedback.

**The readable metadata rule.** Use the explicit muted color for secondary text. Custom palettes retain six roles per appearance mode. Valid fills and accents remain as chosen. Adjust text, metadata and links against each actual painted background, targeting 4.5:1 contrast. Invalid hex colors prevent saving; low contrast does not replace a whole palette.

## Typography

Use Fastframe's default fonts with detected platform rendering. The library and Updates headings use the headline token; details and reviews use the title token. Card names use the card-title token and authors use the label token. egui's strong treatment changes text color rather than selecting a separate bold font. Full names and paths remain available through tooltips.

The brand pairs the original `native/assets/plumbob.svg` at 22 by 34 points with an 18-point Mod Manager label. Discover retains its existing provider typography, readable author documents and monospace preformatted text. Keep native accessibility labels on icon-only controls.

## Layout

At widths of at least 900 points, a fixed 240-point rail (blue in light mode, charcoal in dark mode) contains the brand, destinations, Trash and Settings. Below that width, compact top navigation replaces the rail and Trash remains in the header. The rail has no slogan; a 28-point gap separates the brand from navigation. Content uses 24-point panel margins.

Your mods places Add mods in its title row, followed by manual Check updates for linked mods in windows at least 600 points tall, any actual result summary, search, All/On/Off filters and icon card/list controls. These filters intersect name and filename search without changing files. Your mods, Discover and Updates remain reachable while Settings occupies the central page.

Cards use a 16-point gap, 140-point image and 98-point information/footer region. If no visible mod has a saved preview, the image region is 88 points and cards total 186 points. Columns use `max(1, floor((available_width + 16) / 236))`; remaining width divides evenly. Below 220 points of available content height, cards temporarily render as compact list rows without changing the saved view choice. Rows normally use 64-point height, 44-point previews and 10-point vertical padding. Below 64 points of available list height, rows use 40-point height, 32-point previews and 4-point vertical padding to keep the whole switch visible in short windows. Both views virtualize visible rows.

Selecting a mod opens the same centered details popup at every width. The popup preserves the library behind it, pins its title and actions, and scrolls files and source details together. It leaves 24 points around its frame in narrow windows, has a content width up to 600 points, and reduces its preview from 240 to 120 points in short windows. Close, Escape or backdrop click dismisses it without clearing selection. Previous/Next and Alt+Left/Right follow the filtered order.

Settings retains Save changes, Discard changes and Saved or Not saved status above its scrolling section, with a 1120-point maximum content width. Appearance, Folders, Connections, App updates and Backup tabs keep each task focused. Theme choices and edits stay in a draft with a scoped production-widget preview. The separate Light colors and Dark colors editors do not change window mode. At wide widths the editor and preview sit side by side; narrow windows stack them. Discover uses a Filters button at every size, initially collapsed, showing the active type/category/version count and current sort. Its unindented expanded controls use the full content width and wrap. A concise source-saving and verified-install disclosure sits beside the button and wraps naturally. Result count and displayed range sit left, with Previous/Next aligned right. Virtualized results own a scroll area at every size. Below 600 points of window height, expanded filters use a bounded independent scroll area so Clear filters remains reachable. The responsive project grid remains. Project details retain a readable description column and release-specific dependencies, stacked at narrow widths. See [the app guide](native-app.md) for complete workflows.

Header Activity replaces the permanent bottom status bar. Its count prioritizes undismissed issues, then running tasks, then unread messages; the tooltip reports all three. Compact navigation uses an Activity icon/count and an icon-only appearance toggle. The redundant Local library and CurseForge browser shell labels and scan-duration text are removed; existing heading counts remain.

Activity opens a nonmodal popup anchored to the header with bounded vertical scrolling. Its explicit X uses the existing Clear SVG and has the tooltip and accessibility name Close activity. Outside click, Escape and X close the panel without cancelling work. Running tasks appear first. Browser download import retains its Cancel action, disabled while Installing, and its existing retry flow. Operation history retains scoped Retry and defaults open when an operation has failed.

Brief in-app notifications appear at bottom-right with a 24-point inset, 16-point frame padding, 12-point corners and content width up to 360 points. View activity and Dismiss are explicit actions. Ordinary notices expire after five seconds but remain in session history; reading them in Activity prevents another toast. Repeated identical notices are distinct. Unchanged errors are deduplicated, take priority over routine notices and remain in the foreground until explicitly dismissed. Dismissed errors remain in history. Message history targets 30 entries but never evicts an undismissed error. History is in memory and does not survive restart. Confirmation/review modals suppress both popup and toasts. No operating-system notifications are added.

## Elevation & Depth

Cards use tonal separation and restrained borders without custom shadows. Details and reviews use standard popup shadows over a dimmed backdrop. Menus remain above details. Management reviews temporarily replace details and cancellation returns to the selected mod. Selection and focus draw an immediate 1.5-point outline. Static loading placeholders reserve geometry without shimmer or pulsing.

## Shapes

Cards use the card radius, search uses the search radius, and ordinary widgets use the widget radius. Reviews retain 12-point corners and 24-point padding. Discover cards use the same 12-point corners as library cards. The original plumbob and authored compass add personality without generated mod photography.

## Components

- Cards show real names, saved authors when available, file counts and On/Off switches. Cards and list rows use the same direct `Action::Toggle { enabled }` switch without opening details or a review. Failures remain visible as persistent in-app error notifications and in Activity. Core locking, ownership, collision checks and journaling still apply. External mods retain their migration review. Cards and list rows share clicked-identity context menus.
- Add mods opens Import folder and Import ZIP, followed by desktop picking and the existing import review. New folder and ZIP imports enable in the selected game by default. If enabling is blocked, the app keeps the imported library content and unrelated game files, and shows a warning. Originals stay in place. Trash is recoverable and requires the existing ownership checks.
- Library and Discover share a 48-point search shell with 10-point corners, 12-point horizontal and 6-point vertical margins, a leading search icon and a 36-point text target. Nonempty queries expose a 32-by-32-point clear button that clears text and restores input focus; empty wide fields show Ctrl+F. Focus outlines the entire shell in blue for built-in themes, using the saved accent for custom themes. The input has no redundant hover tooltip. Discover adds an 88-by-36-point blue Search button and submits only through Search or Enter. Library search filters as text changes. Ctrl+F focuses search and dismisses details while preserving selection. Card/list changes retain search and selected identity. Missing or broken photos show No Preview; details use contain sizing and cards use centered destination-aware cover crops.
- Updates shows only mods with explicitly attached CurseForge projects and their actual reports. Check updates is manual. The library summary's Review opens Updates; each report's Review opens the existing mod details popup. Unknown versions, changed files, incomplete checks and failures remain distinct. Recording an installed release requires review and writes metadata only. Update checks never download or replace files.
- Discover uses real provider search, filters, summaries, descriptions, dependencies and supplied images. Cards retain full-width 140-point photos and padded information bodies. Download in browser uses the shared blue primary action and download icon. An inline note discloses source saving and verified automatic installation. Clicking starts nonmodal preparation and opens the official download page without another popup; card bodies open details. Details retain the release selector. Each dependency needs its own explicit action. Source candidate attachment still requires confirmation.
- Reviews retain visible confirmation actions and scroll long paths and bodies. Cancel, Escape and backdrop dismissal approve nothing. Low-confidence source warnings remain visible. Settings applies and persists drafts only through Save changes. Back and Escape preserve drafts; Discard changes also restores removed themes. Invalid hex colors and names prevent saving. Theme JSON and reviewed backup imports stage drafts. Outside Settings, the header switch persists only mode and uses saved custom colors; it hides in Settings.
- Preview requests omit Referer. Failed source candidate and provider detail images hide; card images use No Preview. Mod photos come from user files or actual saved URLs, never bundled generated examples. API keys stay in local settings or ignored environment files and never enter mod metadata.

Discover recognizes local projects through an index of saved, user-confirmed CurseForge project IDs. Known projects replace Downloads with a checkmarked View in library button using the selection fill and readable text; project details also show In your library. Disabled mods count, and local search or On/Off filters do not change recognition. View in library selects the local entry, clears filters that could hide it, and opens its existing details popup. Scans rebuild the index, and changing game folders clears the previous index immediately. Recognition neither attaches a source nor asserts that the saved release is up to date.

Historical Plumbob verification uses `native/verify_navigation.py` for 15 isolated navigation and theme captures, including custom-themed libraries in both modes, 640 by 480 and 1200 by 480, and `verify_updates` for nine synthetic update captures. Capture runs preserve fixture inputs and approve no operations. The finish review found one duplicate inert summary Review on Updates. The final verdict scored its removal resolved after a regression click test and fresh captures. Library summaries keep Review navigation; Updates keeps per-mod Review actions. All eight required repository checks passed, including the release build and packaged workflow verification.

The approved search and charcoal refresh is recorded in `.impeccable/review/search-charcoal/`: discovery captures cover desktop, narrow and 640-by-480 focus/query/empty/provider states; library captures cover light, dark, custom themes and Settings. `real-library/library-dark.png` shows real public covers in an isolated fixture. The independent finish reviewer returned ship with no material fixes for this scope, contingent on documentation and full validation. All eight required repository validation commands passed for this refresh; earlier validation statements above remain historical.

Header Activity evidence: `native/examples/verify_activity.rs` reproduces nine fixture-only captures in `.impeccable/review/header-activity`: `idle-dark`, `waiting-header`, `waiting-open`, `waiting-small`, `waiting-light`, `error-toast`, `error-open`, `success-toast` and `success-expired`. It starts no download and changes no user mods. The independent finish reviewer returned ship with no material fixes and all nine captures valid. `native/tests/activity_ui.rs` covers header/no-footer layout, wide/compact task cancellation, closing without cancellation and persistent error dismissal/history. Activity model tests cover expiration, deduplication, error priority and retention. All eight required repository validation commands passed, and the final Activity tests were rerun after lint cleanup. Earlier review and validation results remain historical.

Discover hides its heading, search, Filters controls, inline consent disclosure and pagination together when scrolling down through results. Scrolling up or entering the quiet buffer near the end of the current results page restores the toolbar. The global app header and navigation remain visible. The toolbar stays visible at the top and on pages that do not scroll. Focused search, expanded filters and an open provider popup hold it visible; Ctrl+F immediately restores and focuses search. Loading, errors and new navigation restore it. Wheel, scrollbar-drag and Page Up/Home or Page Down/End input determine scroll intent. The quiet buffer near the page end keeps the toolbar visible during small downward movements. It applies when maximum results offset minus current offset is at most the current rendered toolbar height plus 96 logical points. Including the rendered height accounts for space freed by collapse, so collapse cannot manufacture a new bottom hit. While inside the buffer, the measured distance from the bottom stays anchored through viewport resize and toolbar reveal. Upward input releases the anchor before scrolling. Downward input can hide the toolbar again only after scrolling above the buffer. Idle layout and animation changes cannot reverse toolbar visibility. Toolbar collapse and reveal use the native cubic-out height animation, capped at 150 ms and respecting a zero animation-time preference. Ctrl+F bypasses the transition to restore and focus search immediately.

Toolbar auto-hide evidence: `native/examples/verify_discovery.rs` states 10–13 cover scrolling down, returning upward and reaching the page end, including small windows, with fixture-only captures in `.impeccable/review/discovery-autohide`. All eight required validation commands passed. The independent animated-toolbar review returned ship with no material fixes across eight valid captures. Earlier passing results are historical.

Bottom-buffer regression evidence: the extended `native/tests/discovery_ui.rs::reaching_bottom_anchors_results_through_the_entire_toolbar_reveal` checks page end, up 60, then down 20. Before the fix, the small downward movement transiently reduced toolbar height to 152.125 instead of 220; after the fix, every frame keeps the full toolbar height at 1200×800, 1870×1006 and 640×480. Up 500 then down 20 verifies hiding resumes outside the buffer. All 27 targeted discovery UI tests passed. Fixture state 15 in `native/examples/verify_discovery.rs` produces `browse-bottom-buffer.png` in `.impeccable/review/discovery-bottom-buffer`. All eight required validation commands passed. Independent review returned ship with no material fixes across four valid captures; this covers the scoped buffer regression, not a replay of the full original video.

Earlier bottom-anchor regression evidence: `native/tests/discovery_ui.rs::reaching_bottom_anchors_results_through_the_entire_toolbar_reveal` reaches the end through direct scrollbar state without wheel momentum. Before the fix it failed with offset 2645 against maximum 2704.875; after the fix it passes at 1200×800, 1870×1006 and 640×480, checking a steady last card throughout reveal and anchor release on upward input. All 27 discovery UI tests passed. Fixture captures are in `.impeccable/review/discovery-bottom-anchor`; `native/examples/verify_discovery.rs` state 14 produces `browse-bottom-anchor.png` from direct end-of-scroll state without wheel momentum and asserts the anchor. All eight required validation commands passed. Independent review returned ship with no material fixes across eight valid captures; the regression verifies the specific bottom-reveal defect, not a replay of the full user recording.

## Do's and Don'ts

- Do keep navigation and management labels short and recognizable.
- Do preserve real photos, readable muted text, keyboard focus and selected identity.
- Do require confirmation for source attachment and reviewed management actions.
- Do require user initiation for downloads and update checks.
- Don't invent photos, authors, provider candidates or counters.
- Don't open details or a confirmation popup when a managed mod's On/Off switch is clicked.
- Allow permanent deletion only of reviewed app-managed mods already in Trash, after an explicit irreversible-deletion acknowledgement and confirmation. Never remove unmanaged files or unrelated system trash.
