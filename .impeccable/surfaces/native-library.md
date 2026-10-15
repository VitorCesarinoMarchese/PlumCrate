# Native mod library

Mode: Operate. User-approved direction: Plumbob, chosen from five HTML concepts and implemented in native egui. The concept reference is `/home/tept/.codex/artifacts/ts4-design-previews/concept-1.html`; actual Rust widgets define behavior. [DESIGN.md](../../DESIGN.md) is the canonical visual guide.

Your mods leads with actual library counts and the blue Add mods menu for folder/ZIP imports. A fixed 240-point blue rail at widths of at least 900 points holds Your mods, Discover, Updates, Trash and Settings. Narrow windows use compact top navigation with Trash in the header. The slogan hides in short windows. Search, All/On/Off filters and icon card/list controls preserve selected identity.

Cards show real previews and metadata. Their photo region is 140 points above a 98-point information/footer region, or 88 points without saved previews. Under 220 points of available content height the library temporarily renders compact list rows without changing the view preference. Selecting a card or row opens the existing centered mod details popup at every width. Right-click selects the clicked identity before showing shared actions. On/Off switches request Action::ReviewToggle and require confirmation before file operations.

The light default uses pale blue panels, white cards and green selection accents; the dark counterpart uses navy. Saved themes and custom light/dark palettes remain intact. The original plumbob SVG and authored compass supply identity. Mod photography comes from real user/provider content, never generated bundled photos.

Updates retains manual checks of explicitly attached CurseForge projects. The actual library summary's Review opens Updates; Review beside a linked report opens mod details. Checks never download or replace files. Recording an installed release requires confirmation and writes metadata only. Browser-assisted installations retain their existing release, source, folder and game-destination review; approval covers verified import and enabling. Desired dependencies require separate reviews. Source attachment never happens silently. Trash remains recoverable.

Settings retains saving, drafts, validation, custom-theme transfer and the existing local credential rules. Preview requests omit Referer; missing/broken local previews show No Preview, while broken provider images hide. Low-confidence source warnings remain visible. All management, provider and filesystem safety requirements remain in force.

Historical capture reports describe prior feature validation, not approval for file operations. Current Plumbob capture and full-validation results belong to the implementation report. The native egui app does not use the HTML/CSS detector.
