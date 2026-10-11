# Product

<!-- impeccable:product-schema 1 -->

## Platform

Linux native desktop, built with Fastframe and egui. This is not an iOS or Android interface. The desktop UI in `native/` uses the Rust domain library in `core/`.

## Users

The Sims 4 players managing their local mods and custom content.

## Product purpose

Browse a local mod library, inspect photos and files, and manage installed state without losing original files or metadata.

## Capabilities and constraints

Preserve all existing native management features: folder/ZIP import, reviewed enable/disable, external migration, rename, explicit source attachment/removal, trash/restore, settings transfer, search and photo previews. CurseForge browsing supports provider search, content types, scoped categories, game versions, sorting, pagination, summaries and screenshots, using a key saved in local Settings. The current browser supports search and inspection. Product scope includes in-app mod downloads and updates initiated by the user. Source candidates must come from real provider data and require user confirmation. Never permanently delete user files or remove unmanaged files.

## Brand commitments

The user requested a replacement design closer to Deadlock Mod Manager and CurseForge, with photo cards as the default library and a list switch. Preserve the Sims 4 Mod Manager name. Build directly in the app rather than generating a mockup.

## Evidence on hand

The current native implementation is in native/src. The migration completion report records filesystem behavior and validation. Existing user mod previews are content, not bundled artwork. Missing photos must show No Preview; never invent mod metadata or source candidates.

## Product principles

- Keep the app local-first.
- Keep management actions explicit and recoverable.
- Prioritize fast library browsing and photo recognition.
- Preserve complete file and source inspection.
