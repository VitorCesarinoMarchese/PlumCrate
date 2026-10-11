---
name: Sims 4 Mod Manager native
description: A photo-first library for local Sims 4 mods.
colors:
  panel-light: "#F6F4EF"
  panel-dark: "#131315"
  surface-light: "#FFFFFF"
  surface-dark: "#1B1B1E"
  text-light: "#1F1E1B"
  text-dark: "#F5F3EE"
  muted-light: "#656056"
  muted-dark: "#B3B1AB"
  selection-fill-light: "#EBDDB9"
  selection-fill-dark: "#3D3423"
  selection-stroke-light: "#533D14"
  selection-stroke-dark: "#E8CF95"
  border-light: "#DAD5C9"
  border-dark: "#39393D"
  widget-fill-light: "#E9E5DC"
  widget-fill-dark: "#252528"
typography:
  headline:
    fontSize: "27pt"
  title:
    fontSize: "23pt"
  card-title:
    fontSize: "15pt"
  body:
    fontSize: "13pt"
  label:
    fontSize: "12pt"
rounded:
  widget: "6pt"
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
    height: "262pt"
  card-dark:
    backgroundColor: "{colors.surface-dark}"
    textColor: "{colors.text-dark}"
    rounded: "{rounded.card}"
    height: "262pt"
  button-primary-light:
    backgroundColor: "{colors.selection-fill-light}"
    textColor: "{colors.text-light}"
    rounded: "{rounded.widget}"
    padding: "8pt 12pt"
  button-primary-dark:
    backgroundColor: "{colors.selection-fill-dark}"
    textColor: "{colors.text-dark}"
    rounded: "{rounded.widget}"
    padding: "8pt 12pt"
---

# Design System: Sims 4 Mod Manager native

## Overview

**Creative North Star: "The local mod library"**

The native library follows the user-approved Deadlock Mod Manager and CurseForge direction. Large previews carry recognition; warm neutral surfaces, gold selection and visible installed state keep local management clear. The light and dark themes share the same hierarchy.

This design applies to the Linux Fastframe/egui app. The Rust implementation is authoritative. Dimensions use egui logical points, not CSS pixels. Fastframe supplies default fonts and platform text rendering.

**Key Characteristics:**

- Photo cards by default, with a compact list switch.
- Warm light and charcoal dark themes with gold selection.
- A centered mod-details popup with the library visible behind it at every width.

## Colors

The frontmatter records the exact built-in theme pairs from `src/ui.rs::setup`; use each pair within its theme. User-created themes override six color roles through `apply_settings_theme` without changing these built-in tokens.

### Primary

Gold selection fill and stroke identify selected navigation, installed state, focused cards and the Import folder action. Selected cards use a gold outline rather than a gold body fill.

### Neutral

Warm panel backgrounds contain the library, header and status. White or charcoal surfaces hold the rail, details popup, search and cards. Main text stays separate from explicit muted text. Warm borders and widget fills provide structure without competing with previews. Hover, active, disabled and error colors otherwise follow egui's theme.

**The readable metadata rule.** Use the explicit muted theme color for secondary text; do not substitute egui's default weak alpha for it.

## Typography

Use Fastframe's default font definitions with detected platform rendering. There is no custom application font family. The library heading is strong at 27 points; the selected-mod title is strong at 23. Card titles are strong at 15. Here, egui's strong treatment selects a text color rather than a separate 700-weight font. Supporting library text uses 13 and metadata uses 12. File basenames use 13 and containing paths use 11. Full names and paths remain available through tooltips where text truncates.

The header displays the supplied blue and green SVG wordmark at 160 points wide, preserving its 2148:482 aspect ratio. Its colors remain the same in every theme. The SVG is embedded from `native/assets/logo.svg`, which is also used by the README. The image has the accessible label and tooltip Sims 4 Mod Manager.

## Layout

The wide library has a fixed 176-point navigation rail. The central panel and details popup use 24-point inner margins; the rail uses 16. The header uses 24 horizontal and 10 vertical points.

Below 1040 points of available width, state tabs replace the rail. Selecting a mod opens a centered details popup while keeping the library visible behind it. Close, Escape, or clicking the backdrop dismisses it without clearing selection. The header and compact local-library status remain available. Imports belong to the library toolbar; selected-mod actions belong to the details popup.

Settings replaces the central library while retaining the header, status and wide navigation rail. Its title, Back to library and Save settings sit above the scroll region. Form content has a maximum width of 720 points. The bottom status uses 24 horizontal / 6 vertical points of inner margin. Operation history sits in an Activity disclosure with a maximum 90-point scroll height.

Cards are 262 points high with a 16-point gap. The column count is `floor((available_width + 16) / 236)`, with at least one column; widths then divide the remaining space evenly. The 236-point column budget includes the gap, so it is not a guaranteed minimum card width. Images occupy 150 points in height with an 8-point inset. Text and footer content use a 14-point horizontal inset. Cards and list rows render only visible rows.

The list uses 64-point rows, 44-point square previews and 12 horizontal / 10 vertical points of inner inset. The details popup has a maximum content width of 600 points and shrinks to leave 24 points around its frame in narrow windows. Its title and actions sit above a height-bounded body, with Close below it. The preview is 180 points high. Its summary scrolls separately, reserving 150 points for file controls where space allows and keeping an 80-point minimum summary viewport. File rows are 48 points high. Global item spacing is 10 horizontal / 8 vertical; buttons use 12 horizontal / 8 vertical padding and a 34-point minimum interaction height.

## Elevation & Depth

Library depth comes from panel/surface contrast and restrained borders. Cards have no custom shadow. Context menus retain egui's standard elevation. Details and review popups use the theme's surface color and standard popup shadow over a dimmed backdrop. The More actions menu stays above the details popup, including after returning from a review. Details yield to pending management operations and review popups; cancellation returns to the selected mod. Changing the game folder or losing the selected mod closes details. Selection and keyboard focus share a 1.5-point gold card outline; hover uses the current egui hovered widget fill. These states update immediately, without a custom selection animation.

## Shapes

Cards use 12-point corners. Search uses 8-point corners. Buttons, list highlights, previews and installed-state containers use 6-point corners. Keep the larger card shape around the full image and metadata group.

## Components

- Photo cards show the real name, saved author when present, file count and Installed or Stored state. Without a saved author they show Managed collection or External collection. Missing and broken previews show No Preview; pending previews show a spinner. Card cover crops account for both source and destination proportions. Details images use contain sizing.
- Search is a 40-point field with an 18-point icon. It filters display names and filenames as text changes. All, Installed and Stored filters intersect the current search. Rail counts describe the full catalog; the library count describes the visible collection. Filtering does not enable or disable files.
- The card/list switch changes presentation while retaining search and selected identity. Standard native hover and focus remain visible. Selection opens the same details popup at every width. Card and row labels leave pointer activation to their parent so clicking text also opens details.
- Import folder is the primary library-toolbar action, beside Import ZIP and Restore from trash. Settings is a selectable header route. The bottom contains status, notices, errors and disclosed Activity history.
- Reviews use centered in-app modal popups, without draggable title bars or resize handles. They have 24-point padding, 12-point corners, a 23-point heading and a content width up to 460 points. Paths wrap and long bodies scroll while the 36-point action row stays visible. Cancel sits beside the primary action on the right. Editable fields use the widget fill. Escape and backdrop clicks dismiss without approval and do not activate the library behind the popup. Candidate and trash reviews transition to their confirmation one at a time. System file pickers remain desktop-owned.
- Enable/Disable stays direct for managed mods in the details popup. More actions includes the toggle, rename, source lookup/attachment/removal and trash. Right-clicking either a card or a list row selects that mod before opening the shared menu. External mods retain reviewed migration. Menu activations explicitly close the popup before opening their next interaction.
- Settings is a full page with persistent Save settings and Discard actions above scrolling Appearance, Game folders, Source lookup and transfer controls. Back to library returns to browsing; wide library-filter navigation also leaves Settings.
- Custom themes store separate light and dark palettes, each with six hex fields and color pickers: Accent, Background, Surface, Text, Muted text and Border. The editor follows the appearance mode and labels the displayed version Light colors or Dark colors. Create custom theme starts with both built-in palettes; Reset colors restores only the displayed version. Copy theme JSON exports both palettes; Import a theme also accepts older single-palette JSON, preserving it in the matching version and supplying a built-in companion. Save settings validates, applies and persists both versions. Built-in and saved-theme selection in Settings applies immediately and needs Save settings for persistence; the header light/dark switch persists only the appearance mode and preserves all unrelated drafts. An unsaved-changes indicator explains that leaving Settings keeps the draft. Discard restores the saved preferences and editor, retaining any header mode change.
- The details popup retains complete file and saved-source inspection, full-path tooltips and Copy feedback. Source changes and destructive management reviews require explicit confirmation. Low-confidence candidate warnings remain visible.
- Preview requests send no Referer header. Relative local paths resolve under the selected Mods folder. Preview caching resets on scans and retains at most 32 URIs. Candidate review is currently text-only; any future candidate images must hide on failure.

## Do's and Don'ts

- Do preserve photo proportions with destination-aware centered cover crops in cards and contain sizing in the details popup.
- Do keep explicit weak text colors readable on panels, cards, widgets and selected fills.
- Do preserve search and selected identity when switching card/list layouts.
- Do show No Preview for missing or broken mod photos and keep weak source-match warnings visible.

- Don't fabricate mod photography, authors, source candidates or discovery counters.
- Don't add download or update actions to this local management library.
- Don't replace native focus and hover behavior with a custom selection animation.
