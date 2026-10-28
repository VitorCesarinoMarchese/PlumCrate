# Agent Rules

Project-specific rules for future coding agents working on PlumCrate.

## Product scope

- Focus the desktop app on Linux.
- Support searching, browsing, downloading, and updating The Sims 4 mods directly within the app.

## Source lookup and CurseForge safety

- Do not paste, request, or store CurseForge API keys in chat.
- Store local API keys only in untracked `.env` or local app settings.
- `.env` / `.env.*` must stay ignored by git.
- Never store CurseForge API keys in per-mod `meta.json`.
- Allow users to download and update mods through the app.
- Require user initiation for downloads and updates.
- Do not auto-attach sources silently.
- Always require explicit user confirmation before saving source candidates.
- Do not treat weak/name-only matches as verified.
- Do not invent source URLs or source candidates.
- If CurseForge API returns no real candidates, show no candidates.
- Candidate URLs must come from real provider/API data, not guessed slugs.
- Manual/search shortcut links, if added later, must be clearly labeled and not attachable as verified candidates.
- Keep CurseForge integration isolated behind source/provider/client modules.
- Keep the app local-first.

## Metadata rules

- Store mod metadata in each managed mod's `meta.json`.
- Persist rich source attachment metadata only after user attach.
- Rich source attachment metadata may include:
  - provider ID
  - source URL
  - title
  - author
  - project ID
  - file ID
  - confidence
  - reasons/evidence
  - `attachedBy: "user"`
  - `attachedAt`
- Do not store API keys in `meta.json`.

## UI/image behavior

- Broken candidate images should hide.
- Broken mod card images should fall back to `No Preview`.
- Preview requests must omit the Referer header.
- Low-confidence warnings must remain visible for weak candidates.

## Filesystem safety

- Never permanently delete user files except app-managed mods already in trash, after an explicit review, acknowledgement that deletion cannot be undone, and user confirmation.
- Permanent deletion must verify app ownership and unchanged reviewed trash contents. Never empty unrelated system trash.
- Never delete unmanaged files.
- Safe removal must move files to trash where applicable.
- App-created symlinks may be removed only when ownership/safety checks pass.

## Development workflow

- Follow TDD: write/update failing tests before implementation.
- Keep commits small and atomic.
- Commit after each logical change.
- Run relevant targeted tests after each change.
- The repository contains only the Fastframe desktop app in `native/` and Rust domain code in `core/`.
- Run full validation before final report:
  - `cargo test --manifest-path core/Cargo.toml --locked`
  - `cargo test --manifest-path native/Cargo.toml --locked`
  - `cargo clippy --manifest-path native/Cargo.toml --locked --all-targets -- -D warnings`
  - `cargo build --manifest-path native/Cargo.toml --release --locked`
  - `python3 -m unittest discover -s native/tests -p 'test_*.py'`
  - `python3 native/verify.py --binary native/target/release/plumcrate`
  - `native/target/release/plumcrate --verify-workflows`
  - `python3 native/verify_package.py --binary native/target/release/plumcrate`
