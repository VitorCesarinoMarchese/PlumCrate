# Native UI polish verification

The alpha UI fixes address the five priority areas from the full-app critique.

| Area | Result |
| --- | --- |
| Settings save boundary | Settings keeps a separate saved snapshot. Save settings commits the draft; Discard restores saved preferences. Header Light/Dark saves only the mode. Leaving Settings retains drafts. Source lookup and settings exports use saved preferences. |
| Accessibility | Cards/List have visible, accessible names. Review inputs, theme fields, swatches, theme JSON and the API-key field connect to their visible labels. Native AccessKit output verifies layout names and review-label relationships. |
| Short details popups | Summary, source and files share one scroll area. The preview shrinks to 120 points in short windows. Actions, navigation and Close remain pinned. File rows stay virtualized, including when scrolling to file 10,000. |
| Operation guidance | Import reviews explain storage, unchanged originals and separate enabling. Migration explains moving files and preserving installed links. Confirmations name their action. Empty candidate results offer manual attachment with review. Saved sources have Open source and Copy URL actions; files have Copy file paths. |
| Browsing | Repeated introductory copy and counts were reduced. Views with no saved previews use shorter cards. Ctrl+F focuses search; Previous/Next and Alt+Left/Right follow filtered order. Help opens task guidance and shortcuts in a popup. |

Custom theme warnings check text, metadata and links against a 4.5:1 threshold. The affected palette uses built-in colors until corrected, preserving the stored palette for editing. Warnings and Reset colors precede the color fields. Built-in links use the theme gold and pass the same contrast test as metadata.

## Validation

All eight repository-required checks passed:

| Check | Evidence |
| --- | --- |
| Core Rust tests | 120 unit tests and 36 integration tests passed. Live provider tests remain opt-in. |
| Native Rust tests | 58 unit tests, 10 management tests and 14 navigation tests passed. |
| Clippy | All native targets passed with warnings denied. |
| Release build | Built `native/target/release/ts4-mod-manager-native` with the locked dependencies. |
| Python tests | All 10 tests passed. |
| Binary verifier | 100, 1,000 and 10,000-mod fixtures passed search, path and unchanged-input checks. The 10,000-mod scan took about 55 ms in this run. |
| Management workflows | 18 operations passed using temporary fixtures, including imports, two-instance toggles, migration, source confirmation and trash/restore. |
| Package verifier | Packaged executable, desktop assets, licenses, catalog inspection and management workflows passed. |

Visual verification inspected 32 native captures together, followed by one confirmation round: 11 library/settings/custom-theme views and 21 popup, recovery and compact-library views. Captures include light/dark, narrow, long-content and 640×480 cases. Details capture checks require an actual open popup, visible Close and at least 15,000 preview pixels. No capture approved an operation or saved settings.

A follow-up preview refinement removes the full-width colored strip behind loaded details images. Square, portrait and landscape images retain their proportions and remain centered; thumbnails and missing-image placeholders keep their backgrounds. The regression test first failed on the empty side gutters, then passed after the fix. All eight checks were rerun. The popup harness captured 23 views, adding square-image cases in light and dark; six affected detail layouts were inspected together, including 640×480 and long titles. See the [square preview capture](images/mod-details-square.png).

The component polish increases tall-window details previews to 240 points and lets shorter entries shrink to their content. Source metadata has tighter spacing, section headings share a 16-point size, source actions use full-height buttons, and Enable uses the selection fill. The footer separates Previous/Next from Close. Regression coverage checks a one-file popup's height and image size, footer placement, exact source URL opening/copying and clipboard feedback. Existing tests still reach file 10,000, preserve full file paths and keep Close visible at 640×480.

All eight checks passed for this component pass. The harness now captures 25 views, adding one-file details with an author in dark and synthetic amber themes. Eight affected layouts were inspected together, followed by one confirmation batch. The first batch verifies light/dark, narrow, long-title and 640×480 layouts; the confirmation verifies the amber palette. The [amber details capture](images/mod-details-amber.png) uses synthetic content, not a downloaded mod.

Escape now returns from Settings to the library without saving or discarding drafts, even while a field is focused or background recovery is pending. It leaves an open menu, Help or management review to handle Escape first. Navigation tests exercise Help → Settings → library, action menu → details, and review cancellation → details, preserving drafts, selection and search without quitting or cancelling background work. All eight required checks passed after this change.

Run the capture tools from a graphical desktop:

```sh
python3 native/verify_navigation.py --binary native/target/release/ts4-mod-manager-native --capture-dir native/target/ui-navigation
cargo run --manifest-path native/Cargo.toml --locked --example verify_popups -- native/target/ui-popups
```

The [native app guide](native-app.md) documents the full validation commands. The README uses the refreshed [library screenshot](images/library-dark.png). The [short-window details capture](images/mod-details-small.png) shows the pinned popup controls.

A real screen-reader walkthrough still needs a manual check; this pass verifies AccessKit metadata and native keyboard interaction.
