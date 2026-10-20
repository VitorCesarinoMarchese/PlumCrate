# Website previews

Run the bilingual marketing website locally with Node.js 22 or later:

```sh
cd website
npm run dev
```

Open <http://localhost:4173> for the selected Gallery website. Open <http://localhost:4173/?previews=1> to compare the original five options. The preview selector switches between Gallery, Home, Night, Guide, and Studio. EN and PT-BR switch all visitor copy. Screenshot tabs show the actual native app in light mode, dark mode, and Settings. Open clean preview hides the concept selector. Query parameters preserve the selected preview and language.

## User guide

The player guide lives at `/guide` in production and `guide.html` locally, in English and Brazilian Portuguese. Landing-page guide links preserve the selected language. It covers installation, game-folder selection, imports, library controls, optional CurseForge access, manual update checks, trash and settings. Desktop control labels remain English to match the current app.

CurseForge sources were checked on 2026-10-20. The official support article describes a developer application reviewed by Overwolf; section 2.2 of the API terms restricts sharing or transferring keys. The guide links to that process without promising a general personal-key entitlement, and describes local configuration only for authorized access. It never collects keys. Local mod management remains usable without a key.

Sources: [application instructions](https://support.curseforge.com/support/solutions/articles/9000208346-about-the-curseforge-api-and-how-to-apply-for-a-key), [API terms](https://support.curseforge.com/support/solutions/articles/9000207405-curse-forge-3rd-party-api-terms-and-conditions), and [EA game settings](https://help.ea.com/en/articles/the-sims/the-sims-4/how-to-install-mods-the-sims-4/).

## Deploy the chosen preview to Vercel

Gallery is live at <https://ts4-mod-manager.vercel.app>. Its production default hides the preview selector. The Vercel project is `ts4-mod-manager` in `vitorcesarinomarcheses-projects`.

From the `website` directory, deploy with your authenticated Vercel account:

```sh
npx vercel deploy --prod --yes --project ts4-mod-manager --scope vitorcesarinomarcheses-projects
```

The checked-in `vercel.json` uses `public` as the output directory and disables the build command. The root `api/release.js` becomes a Node.js function. This deployment uses the CLI; automatic Git deployments are not connected. If importing this repository through Vercel later, set Root Directory to `website` and Framework Preset to Other.

After deploying, verify both languages, all screenshot tabs, and `/api/release` on the production URL.

Vercel references: [project configuration](https://vercel.com/docs/project-configuration), [monorepos](https://vercel.com/docs/monorepos), and [functions](https://vercel.com/docs/functions/quickstart).

## Release downloads

`/api/release` reads the 30 most recent public GitHub releases from `VitorCesarinoMarchese/TS4ModManager`. It chooses the most recently published release containing a non-ARM, non-i686 AppImage. Published prereleases are eligible because the project is in alpha. Drafts, signatures, and download URLs outside the repository are excluded.

When an AppImage exists, the main button points directly to its GitHub asset. The response is cached at Vercel for five minutes with a one-hour stale response allowance. GitHub rate limits or outages fall back to the releases page. When no AppImage exists, the site says so and links to releases. No token is required, and no CurseForge key is used by the website.

## Verify changes

```sh
npm test
node --check public/app.js
node --check api/release.js
```

Guide tests check the visitor destination, section anchors, task coverage, official API sources, and absence of key-collection inputs. The release selection tests cover empty releases, prereleases, draft exclusion, incompatible files, allowed download URLs, and publication ordering. Run the desktop app's checks from AGENTS.md before reporting repository work complete.

## Assets

The three WebP screenshots come from `docs/images/library-light.png`, `library-dark.png`, and `settings.png`. They are actual app captures using an isolated demonstration library and real creator cover artwork. They do not represent a user's installed collection. The site discloses this beside the screenshots. [Original capture procedure and creator credits](../docs/images/README.md).

Each screenshot has an Impeccable provenance sidecar. The plumbob is the existing `native/assets/plumbob.svg`. Manrope is self-hosted from Google Fonts under the included `public/assets/Manrope-LICENSE.txt` license. No visitor data or keys are stored by the site.
