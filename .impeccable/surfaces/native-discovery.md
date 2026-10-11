# Native CurseForge browser

Mode: Operate. Extend the existing Linux egui app with a CurseForge-style search section. Preserve the established charcoal, warm neutral and gold theme, logo, Settings, local library and reviewed management actions.

Thesis: find Sims 4 mods and custom content on CurseForge inside the app, then inspect provider summaries and screenshots before deciding what to do next.

Own world: existing native theme roles, 27-point page heading, compact square project logos, readable summaries, subdued download/date/category metadata, restrained borders and visible native controls. This extends the incumbent system rather than replacing its colors or typography.

First viewport: My mods and Browse mods tabs under the existing logo; Browse mods title and The Sims 4 provider label; full-width search field with Search button; category and sorting selectors; real result count and Previous/Next; a responsive project grid. Four columns at 1200 points and two at 680. At 640×480 the content below the title scrolls together so controls and all results remain reachable.

Story: save a CurseForge key in local Settings, browse popular projects or search by name, choose provider categories and sort, navigate pages, select a project, read its full summary and view its supplied screenshots. A pinned Back to results restores the current page. Escape returns from project details. Ctrl+F returns to browser search. Opening details never launches a website.

Form: code-led native application extension under the user's pinned CurseForge reference. No generated visual assets. Photos, authors, dates, categories, links and counts come only from provider responses. Synthetic data and procedural images are confined to tests and the capture example.

Constraints: browsing never downloads or updates mods, writes mod metadata or attaches sources. Images omit Referer and failed provider images hide. Saved credentials only; unsaved settings drafts do not change provider requests. Search runs on a detached worker with stale-response rejection and coalesced pending requests. No key is available in this environment, so live provider validation remains outstanding.

Finish: inspect native captures, resolve material independent-review findings, document the browser extension, run all repository validation, and verify signed commits if signing succeeds. Existing design files are authoritative; no unrelated drift repair.

Finish evidence: inspected all eight fixture captures in `native/target/ui-discovery`: `browse-dark`, `browse-light`, `browse-narrow`, `browse-small`, `project-dark`, `project-small`, `browse-setup` and `browse-error`. Wide captures show four aligned project columns in both themes; the narrow capture shows two. Missing fixture logos hide without moving the summary or action rows. Small captures show the shared browser scroll region and pinned project return control. Setup and rate-limit captures show their recovery actions. Source inspection confirms keyboard card focus uses the selection stroke, bounded title/summary regions align card rows, and short windows scroll controls with results. The independent reviewer returned ship after those two fixes. Updated `docs/native-design.md` and `.impeccable/design.json` with browser component rules; all incumbent frontmatter tokens remain unchanged. Live provider validation remains outstanding because no API key is available.
