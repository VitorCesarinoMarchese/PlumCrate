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

The frontmatter records the exact built-in theme pairs from `src/ui.rs::setup`; use each pair within its theme. User-created themes override six color roles through `apply_settings_theme` without changing these built-in tokens. Contrast checks cover text and muted text on the background, surface and relevant control fills, plus links. A palette below 4.5:1 uses built-in colors for that mode until corrected; its stored colors remain editable. Warnings and Reset colors provide recovery.

### Primary

Gold links meet the same text contrast requirement as metadata. Gold selection fill and stroke identify selected navigation, installed state, focused cards and the Import folder action. Selected cards use a gold outline rather than a gold body fill.

### Neutral

Warm panel backgrounds contain the library, header and status. White or charcoal surfaces hold the rail, details popup, search and cards. Main text stays separate from explicit muted text. Warm borders and widget fills provide structure without competing with previews. Hover, active, disabled and error colors otherwise follow egui's theme.

**The readable metadata rule.** Use the explicit muted theme color for secondary text; do not substitute egui's default weak alpha for it.

## Typography

Use Fastframe's default font definitions with detected platform rendering. There is no custom application font family. The library heading is strong at 27 points; the selected-mod title is strong at 23. Card titles are strong at 15. Here, egui's strong treatment selects a text color rather than a separate 700-weight font. Supporting library text uses 13 and metadata uses 12. File basenames use 13 and containing paths use 11. Copy file paths identifies the clipboard content; saved sources have explicit Open source and Copy URL actions. Full names and paths remain available through tooltips where text truncates.

The CurseForge browser reuses the 27-point page heading and default fonts. Project cards use 16-point names, 14-point summaries and 12-point metadata. Names wrap to two rows and summaries to three, with full text in tooltips. Project details use a 26-point title and 18-point section headings. Author headings use 20 points for levels one and two, and 17 for deeper levels; description text uses 14. Preformatted text uses monospace and quotes use italics. These sizes belong to the browser components; they do not replace the library type ramp.

The header displays the supplied blue and green SVG wordmark at 160 points wide, preserving its 2148:482 aspect ratio. Its colors remain the same in every theme. The SVG is embedded from `native/assets/logo.svg`, which is also used by the README. The image has the accessible label and tooltip Sims 4 Mod Manager.

## Layout

The wide library has a fixed 176-point navigation rail. The central panel and details popup use 24-point inner margins; the rail uses 16. The header uses 24 horizontal and 10 vertical points.

Below 1040 points of available width, state tabs replace the rail. Selecting a mod opens a centered details popup while keeping the library visible behind it. Close, Escape, or clicking the backdrop dismisses it without clearing selection. The header and compact local-library status remain available. Imports belong to the library toolbar; selected-mod actions belong to the details popup.

Settings replaces the central library while retaining the header, status and wide navigation rail. Its title, Back to library and Save settings sit above the scroll region. Form content has a maximum width of 720 points. The bottom status uses 24 horizontal / 6 vertical points of inner margin. Operation history sits in an Activity disclosure with a maximum 90-point scroll height.

Cards are 262 points high with a 16-point gap. Views without any saved previews use 200-point cards with an 88-point preview placeholder. The column count is `floor((available_width + 16) / 236)`, with at least one column; widths then divide the remaining space evenly. The 236-point column budget includes the gap, so it is not a guaranteed minimum card width. Images occupy 150 points in height with an 8-point inset. Text and footer content use a 14-point horizontal inset. Cards and list rows render only visible rows.

The list uses 64-point rows, 44-point square previews and 12 horizontal / 10 vertical points of inner inset. The details popup has a maximum content width of 600 points and shrinks to leave 24 points around its frame in narrow windows. Its title and actions sit above a height-bounded body, with Close below it. The preview is 240 points high, reduced to 120 in short windows. The body shrinks to its content for shorter entries and scrolls within the available height for longer ones. Saved-source metadata uses four-point spacing, with 16-point section headings. Enable uses the selection fill; source actions use full-height buttons and Copy URL shows clipboard feedback. Footer navigation sits on the left and Close on the right, below a separator. Summary, saved-source actions and files share one scroll region; the title, mod actions and Close remain pinned. File rows are 48 points high and virtualized within that shared region. Global item spacing is 10 horizontal / 8 vertical; buttons use 12 horizontal / 8 vertical padding and a 34-point minimum interaction height.

Browse mods uses the full central panel without the local-library rail. My mods and Browse mods route buttons sit below the logo and use the existing selected fill. The search field is 42 points high, with an 88-by-36-point Search button. Filter labels sit above their selectors. Browse by, Category, Game Version and Sort by use widths of 180, 260, 142 and 155 points in 60-point-high groups. Clear filters uses a 104-point-wide group aligned with the selector row. Groups wrap together as space narrows. Results use a 16-point column gap and `max(1, floor((available_width + 16) / 280))` columns. The 280-point budget includes the gap. Cards have 14-point inner padding, a 238-point minimum content height and a 268-point row budget for virtualization. Fixed 64-point header and 54-point summary regions align the category, date and action rows despite missing logos or different text lengths. Logos fit within 60-by-60-point bounds.

At content heights below 600 points, Filters folds the selectors and shows the active content-type, category and version count. The browser scrolls everything below its heading together, including search, expanded filters, pagination and cards. Clear filters remains reachable by scrolling. Taller windows show the selectors above the virtualized result grid. Project details keep Back to results, Description and Dependencies above one scroll region containing the title, summary, categories, actions, author document, release dependencies and screenshots. The page has a maximum width of 1160 points. At widths of at least 920 points, a description column of up to 720 points sits beside a 340-point dependency column with a 24-point gap. Below that width, the description precedes dependencies. Section buttons jump without animation and remain reachable in short windows. The project logo fits within 80-by-80-point bounds; screenshots fit within the available width, up to 780 by 420 points, preserving proportions. Description images fit their column with a maximum height of 420 points.

## Elevation & Depth

Library depth comes from panel/surface contrast and restrained borders. Cards have no custom shadow. Context menus retain egui's standard elevation. Details and review popups use the theme's surface color and standard popup shadow over a dimmed backdrop. The More actions menu stays above the details popup, including after returning from a review. Details yield to pending management operations and review popups; cancellation returns to the selected mod. Changing the game folder or losing the selected mod closes details. Selection and keyboard focus share a 1.5-point gold card outline; hover uses the current egui hovered widget fill. These states update immediately, without a custom selection animation.

## Shapes

Cards use 12-point corners. Search uses 8-point corners. Buttons, list highlights, previews and installed-state containers use 6-point corners. Keep the larger card shape around the full image and metadata group. CurseForge project cards use the search component's 8-point corners and restrained border; their logos and screenshots use 6-point corners.

## Components

- Photo cards show the real name, saved author when present, file count and Installed or Stored state. Without a saved author they show Managed collection or External collection. Missing and broken previews show No Preview; pending previews show a spinner. Card cover crops account for both source and destination proportions. Details images use contain sizing, centered directly on the popup surface without a full-width background strip.
- Search is a 40-point field with an 18-point icon. It filters display names and filenames as text changes. All, Installed and Stored filters intersect the current search. Rail counts describe the full catalog; the library heading count describes the visible collection. Ctrl+F focuses search and dismisses open details while retaining selection. Filtering does not enable or disable files.
- The visibly labeled Cards/List switch changes presentation while retaining search and selected identity. Standard native hover and focus remain visible. Selection opens the same details popup at every width. Previous/Next and Alt+Left/Right follow the filtered order without returning to the library. Card and row labels leave pointer activation to their parent so clicking text also opens details.
- Import folder is the primary library-toolbar action, beside Import ZIP, Restore from trash, and Check updates. Settings is a selectable header route. The bottom contains status, notices, errors and disclosed Activity history. Help in the header (F1) opens task guidance and keyboard shortcuts in a centered popup.
- Reviews use centered in-app modal popups, without draggable title bars or resize handles. They have 24-point padding, 12-point corners, a 23-point heading and a content width up to 460 points. Paths wrap and long bodies scroll while the 36-point action row stays visible. Cancel sits beside the primary action on the right. Editable fields use the widget fill and accessible associations with their visible labels. Confirmation buttons name the approved action. Local folder/ZIP import reviews explain managed storage, preservation of originals and separate enabling; migration reviews explain moving files and retaining installed links. Empty source results offer a manual URL route without inventing matches. Escape and backdrop clicks dismiss without approval and do not activate the library behind the popup. Candidate and trash reviews transition to their confirmation one at a time. System file pickers remain desktop-owned.
- Enable/Disable stays direct for managed mods in the details popup. More actions includes the toggle, rename, source lookup/attachment/removal and trash. Right-clicking either a card or a list row selects that mod before opening the shared menu. External mods retain reviewed migration. Menu activations explicitly close the popup before opening their next interaction.
- Settings is a full page with persistent Save settings and Discard actions above scrolling Appearance, Game folders, Source lookup and transfer controls. Back to library returns to browsing; wide library-filter navigation also leaves Settings.
- Custom themes store separate light and dark palettes, each with six hex fields and color pickers: Accent, Background, Surface, Text, Muted text and Border. The editor follows the appearance mode and labels the displayed version Light colors or Dark colors. Create custom theme starts with both built-in palettes; Reset colors restores only the displayed version. Copy theme JSON exports both palettes; Import a theme also accepts older single-palette JSON, preserving it in the matching version and supplying a built-in companion. Save settings validates, applies and persists both versions. Built-in and saved-theme selection in Settings applies immediately and needs Save settings for persistence; the header light/dark switch persists only the appearance mode and preserves all unrelated drafts. An unsaved-changes indicator explains that leaving Settings keeps the draft. Discard restores the saved preferences and editor, retaining any header mode change.
- Update checks use existing theme colors and native controls. Check updates sits in the library toolbar with a pending spinner and a result summary below it. Only checked mods with newer public releases receive Update available in cards and list rows. The details Updates section has Check this mod, distinct unknown-version, changed-files, unavailable-release, incomplete-history and failure text, and an explicit Open mod page button using the provider URL. Record installed release expands a provider-release selector; Review installed release opens the existing confirmation popup. The final Record installed release action writes metadata only. Notices compare stable public project-release dates, with a visible warning that releases may target other components or game versions. Checks are manual and require a saved local key and an explicitly attached CurseForge project. Seven synthetic production-widget captures in `.impeccable/review/updates-*.png` passed independent finish review with a ship disposition; captures approved no operations.
- The details popup retains complete file and saved-source inspection, full-path tooltips and Copy feedback. Source changes and destructive management reviews require explicit confirmation. Low-confidence candidate warnings remain visible.
- Preview requests send no Referer header. Relative local paths resolve under the selected Mods folder. Preview caching resets on scans and retains at most 32 URIs. Candidate review is currently text-only; any future candidate images must hide on failure.

- CurseForge browsing uses the existing theme colors and native controls. Browse by lists the provider's Sims 4 content types. Category is scoped to that type and shows searchable parent paths; changing type clears the category. Game Version lists searchable numeric versions, newest first. Relevancy, Popularity, Latest update, Date released, Total Downloads and A-Z are provider sort choices. Relevancy leaves ranking to the provider; Date released describes the public API's release-date sort. Search or Enter submits the query; filter and sort changes restart at the first page. Clear filters resets type, category and version while retaining search text and sort. Counts, ranges and Previous/Next state come from provider pagination, retaining every submitted dimension. Loading, no-results, missing-key and request-error states replace the grid with specific guidance. Missing-key setup offers Open Settings; request errors also offer Try again. Filter-option failures show a visible warning without hiding available projects.
- Project cards show provider names, authors, summaries, categories, download counts and dates. Missing authors or dates have explicit fallback text. Clicking the card or View details opens its in-app detail page. Hover and keyboard focus draw the existing selection stroke inside the card border. Back to results or Escape restores the current result page; Ctrl+F returns to browser search. Open on CurseForge and Copy link use the supplied HTTPS URL. Browser images omit Referer, reserve static placeholder space while pending and hide on failure. Browsing uses only the API key saved in local Settings. Search and inspection leave files and source attachments unchanged; Download via website opens a separate installation review.

- Project descriptions render ordered native blocks with author text, headings, lists, table rows, images, and safe links beside their originating block. HTML scripts and styles do not run. Empty descriptions and unavailable data have distinct messages. Description, releases, and dependency project lookups retain separate retry actions. Release-specific relation labels distinguish required, optional, tool, incompatible, embedded, included, and unknown entries. Author-linked mods appear under their own explanation and require exact provider lookup before a separate review. No installed-name badge implies dependency satisfaction.
- Static loading placeholders use the existing inactive widget fill and six-point corners without shimmer or pulsing. Browse cards and an initial library scan without entries reserve two rows of inert card geometry. Project descriptions, release selectors, dependency rows, provider images, and pending release reviews reserve their own geometry. Existing library entries remain visible during rescans. Twelve synthetic production-widget captures in `.impeccable/review/details-*.png` passed independent visual review with a ship disposition and no material fixes. They establish appearance, not download consent. The capture command is in [the app guide](native-app.md#read-a-project-and-review-its-dependencies).

- Browser-assisted installation extends the existing review popup and bottom status. Download via website… in project details opens a 23-point-heading review with real release selection, a Downloads-folder picker, source-and-install consent, and the selected game destination. The destination and Start browser download / Cancel row stay below the scroll region and visible at 640 by 480. Release details and Downloads controls scroll. The review uses the existing charcoal or warm light surface, gold primary action, 12-point corners and standard popup shadow. No tokens or raster assets change.
- After approval, the review closes and persistent bottom status reports waiting, queued installation or finishing in the approved game. It shows the watched folder and Cancel download import while cancellation remains available. Initial approval covers source attachment, verified import and automatic enabling. Original downloads stay in place and occupied game paths cause a safe error. Six synthetic production-widget captures in `.impeccable/review/browser-import-*.png` cover both themes, a user window and short review/waiting states. Independent finish review returned ship with no material fixes; captures downloaded and installed no mod files. The capture command and user workflow are in [the app guide](native-app.md#install-from-a-browser-download).

## Do's and Don'ts

- Do preserve photo proportions with destination-aware centered cover crops in cards and contain sizing in the details popup.
- Do keep explicit weak text colors readable on panels, cards, widgets and selected fills.
- Do preserve search and selected identity when switching card/list layouts.
- Do show No Preview for missing or broken mod photos and keep weak source-match warnings visible.
- Do require user initiation for mod downloads and update checks.

- Don't fabricate mod photography, authors, source candidates or discovery counters.
- Don't replace native focus and hover behavior with a custom selection animation.
