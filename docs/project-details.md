# CurseForge project-details contracts

The project-details page combines an author document with dependencies declared by individual release files. Inspection is read-only. Downloads use the existing [browser-installation consent and transaction](browser-downloads.md). The [app guide](native-app.md#read-a-project-and-review-its-dependencies) describes the user flow.

## Author document

`core/src/curseforge_details.rs` reads the provider's description endpoint and parses its HTML fragment with an HTML5 parser. It produces ordered native blocks for paragraphs, headings, list items, quotes, preformatted text, table rows, rules, and images. Ordinary text collapses whitespace; preformatted blocks retain interior whitespace. Table cells become text separated by ` | `. Links appear beside their originating block. The document preserves author wording rather than replacing the description with the browse summary.

Scripts, styles, embedded frames, objects, forms, templates, and hidden elements contribute no content. No website code runs. Links and image sources must resolve to HTTPS URLs with a host and without credentials or control characters. Relative URLs resolve against the provider project URL. Unsupported links retain their visible text without an actionable URL. Images use the existing cache, omit Referer, reserve static space while pending, and hide on failure. Nonempty image alt text remains visible.

The response limit is 2 MiB, and the HTML string limit is 1 MiB. Traversal rejects nesting deeper than 64 levels, more than 20,000 visited elements, or a block count above 10,000 during traversal. Invalid JSON, a missing string, or an exceeded bound produces a lookup failure and retry. The HTML5 parser tolerates malformed markup within these limits. An empty resulting document shows "The author has not provided a description."

## File-declared dependencies

`core/src/browser_import.rs` stores `FileDependencies` on each eligible `BrowserRelease`. The release selector therefore changes the displayed requirements. Eligible files retain the browser-installation restrictions on public stable status, supported extension, length, and published checksum. The selector is not a complete list of every provider release.

| Provider relation | Label | Separate download review |
| --- | --- | --- |
| 1 | Embedded library | No |
| 2 | Optional | Yes, after project lookup |
| 3 | Required | Yes, after project lookup |
| 4 | Tool | Yes, after project lookup |
| 5 | Incompatible | No; visible warning |
| 6 | Included | No |
| Other numeric value | Unknown relation | No |

Missing or non-array dependency data is unknown. Malformed entries are skipped and make the list partial. The parser examines at most 128 entries; longer lists are partial. It deduplicates identical project-ID and relation pairs. A complete empty list means no API-declared dependencies for that file. The UI still directs users to author instructions for additional requirements.

Dependency rows resolve provider project IDs through the API and validate the returned ID, Sims 4 game ID, title, and usable project URL. Failed lookups retain their relation and project ID with a retry. No title or installed filename comparison proves that a dependency is satisfied. Provider project relations do not identify a guaranteed compatible release.

## Author-linked mods

Supported CurseForge Sims 4 project URLs in the description appear separately as author-linked mods. They remain links rather than declared dependencies. **Inspect linked mod** searches the exact slug, requires one exact project-path match, and validates the returned project identity through a second API lookup. Ambiguous, missing, or mismatched evidence cannot offer a download review. The app never constructs guessed project URLs or uses name-only matches.

## Native ownership and consent

`native/src/project_details.rs` owns independent description, release, dependency-reference, and author-link lookup state. Separate workers perform bounded provider reads. Selection and saved-key identity scope results; late responses from another project, key, or selected file cannot replace the current information. Description and release failures have separate retries.

`native/src/discovery.rs` renders the document, release-specific rows, section navigation, and review actions. A parent download request carries both the loaded provider project and selected file ID into `native/src/controls.rs`. The review verifies that the file belongs to that project and shows dependencies for its current selection. Dependency and author-linked actions instead request their own provider project and release review.

Every install requires **Start browser download**. The approval covers only the reviewed source, file, Downloads folder, and game. Dependencies share the existing verified import, enabling, collision, cancellation, and ownership checks. One browser installation can run at a time. There is no recursive graph, automatic dependency download queue, direct API file download, or automatic replacement of older managed releases. Reading descriptions and inspecting relations save no source metadata.

`native/src/skeleton.rs` supplies static placeholders using existing theme fills. They cover browse cards, an initial library scan without entries, independently pending detail sections, provider images, and pending release reviews. Local mod images retain their existing preview behavior.

## Evidence

`native/examples/verify_project_details.rs` renders twelve production-widget captures with synthetic metadata. `.impeccable/review/details-*.png` covers dark and light themes, 1871-by-1006, 680-by-760, and 640-by-480 windows, section navigation, loading, empty descriptions, and lookup failures. The example makes no provider requests and installs no files. Its independent visual review returned ship with no material fixes.

`core/examples/verify_project_details.rs` is an opt-in read-only live metadata probe. It uses an already configured local `CURSEFORGE_API_KEY` and a numeric project ID, then reports document, release, and reference counts. It downloads no mod files and saves no metadata. Credentials remain local and untracked. Targeted core and native tests cover parsing, file identity, stale results, input behavior, and consent separately from the visual captures.
