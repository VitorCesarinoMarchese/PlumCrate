# Website previews

Run the bilingual marketing website locally with Node.js 22 or later:

```sh
cd website
npm run dev
```

Open <http://localhost:4173> for the selected Gallery website. Open <http://localhost:4173/?previews=1> to compare the original five options. The preview selector switches between Gallery, Home, Night, Guide, and Studio. EN and PT-BR switch all visitor copy. Screenshot tabs show the actual native app in light mode, dark mode, and Settings. Open clean preview hides the concept selector. Query parameters preserve the selected preview and language.

## Deploy the chosen preview to Vercel

The owner selected Gallery. Its production default hides the preview selector. Deployment is pending.

1. Import the GitHub repository into Vercel. Set the Root Directory to `website` and Framework Preset to Other. The checked-in `vercel.json` uses `public` as the output directory and disables the build command. The root `api/release.js` becomes a Node.js function.
2. Deploy, then verify the selected layout, both languages, all screenshot tabs, and `/api/release` on the deployed URL.

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

The release selection tests cover empty releases, prereleases, draft exclusion, incompatible files, allowed download URLs, and publication ordering. Run the desktop app's checks from AGENTS.md before reporting repository work complete.

## Assets

The three WebP screenshots come from `docs/images/library-light.png`, `library-dark.png`, and `settings.png`. They are actual app captures using an isolated demonstration library and real creator cover artwork. They do not represent a user's installed collection. The site discloses this beside the screenshots. [Original capture procedure and creator credits](../docs/images/README.md).

Each screenshot has an Impeccable provenance sidecar. The plumbob is the existing `native/assets/plumbob.svg`. Manrope is self-hosted from Google Fonts under the included `public/assets/Manrope-LICENSE.txt` license. No visitor data or keys are stored by the site.
