# Mod update checks

The app checks stable public CurseForge project releases and opens the author's project page for manual downloads. It compares publication dates after the user confirms their installed release. A newer project release may target another component or game version. Checks run only on user request and report results inside the app. They do not send operating-system notifications or run automatically at startup. The [app guide](native-app.md#check-for-mod-updates) describes the manual workflow. [Browser-assisted installation](browser-downloads.md) is a separate path that imports and enables an approved download. It does not replace an older managed release automatically.

## Why installed identity needs confirmation

Existing source attachments identify a project. Their legacy `fileId` comes from a source suggestion and does not prove which archive the user installed. Checks ignore that value. The user selects an actual provider release and confirms it before the update workflow writes `sourceAttachment.installedRelease` in the mod's `meta.json`. Browser-assisted imports establish the same baseline from verified downloaded bytes, with source and installation consent in their initial review.

That record contains the project ID, file ID, confirmation time, and a SHA-256 digest of the managed `files` tree. Changing file bytes, names, additions, or removals makes the recorded version unknown until the user confirms again. Metadata and enablement links outside `files` do not affect the digest. Changing or removing the attached project clears the record.

## Ownership and contracts

`core::mod_updates::check_mod_updates(managed_root, mod_id, client)` reads metadata, hashes current content, validates the real provider project page, fetches metadata for the exact recorded provider file, and lists releases. It returns an `UpdateReport` without writing files. Listing stops after ten pages of fifty files. Incomplete or inconsistent history cannot establish that no newer release exists. A strictly later stable public release can establish availability even if the remaining history is incomplete.

`core::mod_updates::confirm_installed_release(managed_root, mod_id, report, release)` acquires the existing operation guard, rechecks the attached source and local digest, and atomically saves the selected provider release. An identical confirmation is idempotent.

Native `Controls` owns `UpdateState`, saved credentials, and the existing review dialog. `management::Action::CheckUpdates` runs on the detached read-only provider queue. It caches successful HTTP responses within one batch to avoid fetching shared projects repeatedly. `Action::RecordRelease` runs on the serialized filesystem queue after explicit confirmation. The selector and review do not write metadata; only the final Record installed release confirmation does. Open mod page opens the validated provider URL without modifying files. Root, catalog generation, saved-key changes, and local operations invalidate pending results. Selection changes preserve library results.

## Design decision

Two designs were compared: controller-owned results using the existing queues, and catalog-owned results with a separate service and provider abstraction. The first fits the current saved-settings and confirmation ownership with fewer layers. The implementation incorporates the second proposal's request deduplication and distinct failed-check state. The update-check implementation adds no download, extraction, or installation path. Browser-assisted installation has its own provider, watcher, and transaction modules.

Core tests cover version identity, date comparisons, pagination, changed content, unsafe files, source replacement, and confirmation. Native tests exercise stale completion rejection, worker independence, real pointer input, mod-page URLs, unchanged files, and the final confirmation button. `verify_updates` captures production controls with synthetic provider data in both themes and short windows.
