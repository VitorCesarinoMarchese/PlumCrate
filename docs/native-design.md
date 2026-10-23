---
name: Sims 4 Mod Manager native
description: Plumbob, a simple Sims-inspired home for local mods.
colors:
  accent-light: "#1263CA"
  accent-dark: "#9BC7FF"
  panel-light: "#EDF4F8"
  panel-dark: "#122C49"
  surface-light: "#FFFFFF"
  surface-dark: "#1D3A57"
  text-light: "#173452"
  text-dark: "#F4F8FB"
  muted-light: "#52687C"
  muted-dark: "#BED0DF"
  border-light: "#D4E0E9"
  border-dark: "#405C75"
  selection-fill-light: "#DFECCB"
  selection-fill-dark: "#2F4E2A"
  selection-stroke-light: "#2E5710"
  selection-stroke-dark: "#CCEBA2"
  widget-fill-light: "#FFFFFF"
  widget-fill-dark: "#1D3A57"
  hover-light: "#E1EDF5"
  hover-dark: "#2D4D6C"
  rail: "#0863B9"
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
  search: "8pt"
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
---

# Design System: Sims 4 Mod Manager native

## Overview

**Creative North Star: "Plumbob"**

The user chose Plumbob from five concepts and approved its implementation. A blue navigation rail, pale blue background, white cards and green installed-state accents give the app a Sims-inspired identity. Keep words short, use recognizable icons, and let actual mod photos carry the library's personality. The navy dark counterpart keeps the same hierarchy.

This guide describes the Linux Fastframe/egui app. `native/src/settings.rs::ThemeColors::base` and `native/src/ui.rs::base_style` define the built-in colors. Rust widgets are authoritative; dimensions use egui logical points. Fresh settings default to light. Existing saved appearance choices and custom themes remain intact.

**Key Characteristics:**

- Blue navigation with Your mods, Discover and Updates.
- Photo cards and compact list rows with direct On/Off switches.
- Short labels, original plumbob and compass icons, and recoverable management actions.

## Colors

### Primary

Blue identifies navigation, links and the Add mods title-menu action. The rail keeps its blue fill in both built-in themes. The built-in Add mods trigger uses the light accent with white text in both modes; its menu returns to the content theme. Active custom palettes also style the rail, its selected destination, Add mods, and mod switches. The custom surface and text colors style the rail, its selection uses the existing selection fill, and the accent styles the primary action and enabled switches. Action text and switch knobs start with the surface color and adjust for readability against their painted fills. Switching back to a built-in theme restores Plumbob colors.

### Secondary

Green selection fills and strokes identify selection and installed state. Use the selected-card outline for focus and selection, without recoloring the entire photo.

### Neutral

Pale blue panels and white cards form the light theme. Navy panels and blue surfaces form the dark counterpart. Main text, explicit muted text and borders come from the paired tokens above. Hover fills provide immediate feedback.

**The readable metadata rule.** Use the explicit muted color for secondary text. Custom palettes retain six roles per appearance mode. Valid fills and accents remain as chosen. Adjust text, metadata and links against each actual painted background, targeting 4.5:1 contrast. Invalid hex colors prevent saving; low contrast does not replace a whole palette.

## Typography

Use Fastframe's default fonts with detected platform rendering. The library and Updates headings use the headline token; details and reviews use the title token. Card names use the card-title token and authors use the label token. egui's strong treatment changes text color rather than selecting a separate bold font. Full names and paths remain available through tooltips.

The brand pairs the original `native/assets/plumbob.svg` at 22 by 34 points with an 18-point Mod Manager label. Discover retains its existing provider typography, readable author documents and monospace preformatted text. Keep native accessibility labels on icon-only controls.

## Layout

At widths of at least 900 points, a fixed 240-point blue rail contains the brand, destinations, Trash and Settings. Below that width, compact top navigation replaces the rail and Trash remains in the header. The slogan A little more you. appears only when the rail has at least 600 points of available height. Content uses 24-point panel margins.

Your mods places Add mods in its title row, followed by manual Check updates for linked mods in windows at least 600 points tall, any actual result summary, search, All/On/Off filters and icon card/list controls. These filters intersect name and filename search without changing files. Your mods, Discover and Updates remain reachable while Settings occupies the central page.

Cards use a 16-point gap, 140-point image and 98-point information/footer region. If no visible mod has a saved preview, the image region is 88 points and cards total 186 points. Columns use `max(1, floor((available_width + 16) / 236))`; remaining width divides evenly. Below 220 points of available content height, cards temporarily render as compact list rows without changing the saved view choice. Rows normally use 64-point height, 44-point previews and 10-point vertical padding. Below 64 points of available list height, rows use 40-point height, 32-point previews and 4-point vertical padding to keep the whole switch visible above footer notices. Both views virtualize visible rows.

Selecting a mod opens the same centered details popup at every width. The popup preserves the library behind it, pins its title and actions, and scrolls files and source details together. It leaves 24 points around its frame in narrow windows, has a content width up to 600 points, and reduces its preview from 240 to 120 points in short windows. Close, Escape or backdrop click dismisses it without clearing selection. Previous/Next and Alt+Left/Right follow the filtered order.

Settings retains Save changes, Discard changes and Saved or Not saved status above its scrolling section, with a 1120-point maximum content width. Appearance, Folders, Connections, App updates and Backup tabs keep each task focused. Theme choices and edits stay in a draft with a scoped production-widget preview. The separate Light colors and Dark colors editors do not change window mode. At wide widths the editor and preview sit side by side; narrow windows stack them. The bottom status retains notices, errors and disclosed Activity history. Discover keeps wrapping filters, real provider pagination and its responsive project grid; short windows fold filters and scroll expanded controls with the results. Project details retain a readable description column and release-specific dependencies, stacked at narrow widths. See [the app guide](native-app.md) for complete workflows.

## Elevation & Depth

Cards use tonal separation and restrained borders without custom shadows. Details and reviews use standard popup shadows over a dimmed backdrop. Menus remain above details. Management reviews temporarily replace details and cancellation returns to the selected mod. Selection and focus draw an immediate 1.5-point outline. Static loading placeholders reserve geometry without shimmer or pulsing.

## Shapes

Cards use the card radius, search uses the search radius, and ordinary widgets use the widget radius. Reviews retain 12-point corners and 24-point padding. Discover cards use the same 12-point corners as library cards. The original plumbob and authored compass add personality without generated mod photography.

## Components

- Cards show real names, saved authors when available, file counts and On/Off switches. Cards and list rows use the same direct `Action::Toggle { enabled }` switch without opening details or a review. Failures remain visible in the bottom status. Core locking, ownership, collision checks and journaling still apply. External mods retain their migration review. Cards and list rows share clicked-identity context menus.
- Add mods opens Import folder and Import ZIP, followed by desktop picking and the existing import review. New folder and ZIP imports enable in the selected game by default. If enabling is blocked, the app keeps the imported library content and unrelated game files, and shows a warning. Originals stay in place. Trash is recoverable and requires the existing ownership checks.
- Search filters as text changes. Ctrl+F focuses search and dismisses details while preserving selection. Card/list changes retain search and selected identity. Missing or broken photos show No Preview; details use contain sizing and cards use centered destination-aware cover crops.
- Updates shows only mods with explicitly attached CurseForge projects and their actual reports. Check updates is manual. The library summary's Review opens Updates; each report's Review opens the existing mod details popup. Unknown versions, changed files, incomplete checks and failures remain distinct. Recording an installed release requires review and writes metadata only. Update checks never download or replace files.
- Discover uses real provider search, filters, summaries, author descriptions, dependencies and supplied images. Its cards place a full-width, 140-point photo above the title and author, using the library's centered cover crop. Summary, category, download count, date and a full-width Downloads button remain below in a padded body. Downloads uses the shared primary action colors, a download icon and visible hover/focus outlines. Clicking it opens the existing release review without opening details; clicking the photo or body opens details. Missing or broken card photos show No Preview in the reserved image area. Static loading cards reserve the same geometry. Browser-assisted installation retains one confirmation for the release, source, Downloads folder and selected game, with the latest eligible release selected by default. After that approval, verified import and enabling follow the existing workflow without further app prompts. Every desired dependency needs its own review. Source search never attaches a candidate silently.
- Reviews retain visible confirmation actions and scroll long paths and bodies. Cancel, Escape and backdrop dismissal approve nothing. Low-confidence source warnings remain visible. Settings applies and persists drafts only through Save changes. Back and Escape preserve drafts; Discard changes also restores removed themes. Invalid hex colors and names prevent saving. Theme JSON and reviewed backup imports stage drafts. Outside Settings, the header switch persists only mode and uses saved custom colors; it hides in Settings.
- Preview requests omit Referer. Failed source candidate and provider detail images hide; card images use No Preview. Mod photos come from user files or actual saved URLs, never bundled generated examples. API keys stay in local settings or ignored environment files and never enter mod metadata.

Discover recognizes local projects through an index of saved, user-confirmed CurseForge project IDs. Known projects replace Downloads with a checkmarked View in library button using the selection fill and readable text; project details also show In your library. Disabled mods count, and local search or On/Off filters do not change recognition. View in library selects the local entry, clears filters that could hide it, and opens its existing details popup. Scans rebuild the index, and changing game folders clears the previous index immediately. Recognition neither attaches a source nor asserts that the saved release is up to date.

Plumbob verification uses `native/verify_navigation.py` for 15 isolated navigation and theme captures, including custom-themed libraries in both modes, 640 by 480 and 1200 by 480, and `verify_updates` for nine synthetic update captures. Capture runs preserve fixture inputs and approve no operations. The finish review found one duplicate inert summary Review on Updates. The final verdict scored its removal resolved after a regression click test and fresh captures. Library summaries keep Review navigation; Updates keeps per-mod Review actions. All eight required repository checks passed, including the release build and packaged workflow verification.

## Do's and Don'ts

- Do keep navigation and management labels short and recognizable.
- Do preserve real photos, readable muted text, keyboard focus and selected identity.
- Do require confirmation for source attachment and reviewed management actions.
- Do require user initiation for downloads and update checks.
- Don't invent photos, authors, provider candidates or counters.
- Don't open details or a confirmation popup when a managed mod's On/Off switch is clicked.
- Don't permanently delete user files or remove unmanaged files.
