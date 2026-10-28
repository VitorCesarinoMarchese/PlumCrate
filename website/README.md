# Website previews

Run the bilingual marketing website locally with Node.js 22 or later:

```sh
cd website
npm ci
npm run dev
```

Open <http://localhost:4173> for the selected Gallery website. Open <http://localhost:4173/?previews=1> to compare the original five options. The preview selector switches between Gallery, Home, Night, Guide, and Studio. EN and PT-BR switch all visitor copy. Screenshot tabs show the actual native app in light mode, dark mode, and Settings. Open clean preview hides the concept selector. Query parameters preserve the selected preview and language.

## User guide

The [player guide](https://plumcrate.vercel.app/guide) lives at `/guide` in English and `/pt-br/guide` in Brazilian Portuguese, locally and in production. Landing-page guide links preserve the selected language. It covers installation, game-folder selection, imports, library controls, optional CurseForge access, manual update checks, trash and settings. Desktop control labels remain English to match the current app. The top navigation stays visible while scrolling. Both pages expose the same static navigation, checked against `public/site-navigation.js`. The contents menu follows the reading position and shows the current section as a subdued, disabled link; other sections remain clickable.

CurseForge sources were checked on 2026-10-20. The official support article describes a developer application reviewed by Overwolf; section 2.2 of the API terms restricts sharing or transferring keys. The guide links to that process without promising a general personal-key entitlement, and describes local configuration only for authorized access. It never collects keys. Local mod management remains usable without a key.

Sources: [application instructions](https://support.curseforge.com/support/solutions/articles/9000208346-about-the-curseforge-api-and-how-to-apply-for-a-key), [API terms](https://support.curseforge.com/support/solutions/articles/9000207405-curse-forge-3rd-party-api-terms-and-conditions), and [EA game settings](https://help.ea.com/en/articles/the-sims/the-sims-4/how-to-install-mods-the-sims-4/).

## Deploy the chosen preview to Vercel

Gallery is live at <https://plumcrate.vercel.app>. The previous `ts4-mod-manager.vercel.app` host redirects permanently to the same path on the PlumCrate host. Its production default hides the preview selector. The Vercel project is `plumcrate` in `vitorcesarinomarcheses-projects`.

From the `website` directory, deploy with your authenticated Vercel account:

```sh
npx vercel link --yes --project plumcrate --scope vitorcesarinomarcheses-projects
npx vercel deploy --prod --yes --scope vitorcesarinomarcheses-projects
```

The checked-in `vercel.json` runs `npm run build` and publishes `dist`. `build.mjs` renders English templates and the existing Portuguese translations in `copy/pt-BR.mjs` into four static pages with localized metadata, navigation, and language alternatives. Edit those sources and rebuild; `dist` is ignored by git. `npm run dev` also builds once at startup. Restart it after editing a source. The root `api/release.js` becomes a Node.js function. This deployment uses the CLI; automatic Git deployments are not connected. If importing this repository through Vercel later, set Root Directory to `website` and Framework Preset to Other.

After deploying, verify both languages, all screenshot tabs, and `/api/release` on the production URL.

Vercel references: [project configuration](https://vercel.com/docs/project-configuration), [monorepos](https://vercel.com/docs/monorepos), and [functions](https://vercel.com/docs/functions/quickstart).

## Release downloads

`/api/release` reads the 30 most recent public GitHub releases from `VitorCesarinoMarchese/PlumCrate`. It chooses the most recently published release containing a non-ARM, non-i686 AppImage. Published prereleases are eligible because the project is in alpha. Drafts, signatures, and download URLs outside the repository are excluded.

When an AppImage exists, the main button points directly to its GitHub asset. The response is cached at Vercel for five minutes with a one-hour stale response allowance. GitHub rate limits or outages fall back to the releases page. When no AppImage exists, the site says so and links to releases. No token is required, and no CurseForge key is used by the website.

## Verify changes

```sh
npm test
npm run build
npx playwright install chromium
npm run verify
node --check public/app.js
node --check api/release.js
```

Navigation tests cover long sections, reverse scrolling, the initial section and the document end. Guide tests check the visitor destination, section anchors, task coverage, official API sources, and absence of key-collection inputs. The release selection tests cover empty releases, prereleases, draft exclusion, incompatible files, allowed download URLs, and publication ordering. `npm run verify` starts a temporary local server and checks all four pages at desktop and mobile widths, with and without JavaScript. It checks language switching, legacy language links, guide sections, titles, screenshots, and preview state. GitHub release responses are fixtures. Captures go to the ignored `.impeccable/review/google-seo` directory. Run the desktop app's checks from AGENTS.md before reporting repository work complete.

## Assets

The three WebP screenshots come from `docs/images/library-light.png`, `library-dark.png`, and `settings.png`. They are actual app captures using an isolated demonstration library and real creator cover artwork. They do not represent a user's installed collection. The site discloses this beside the screenshots. [Original capture procedure and creator credits](../docs/images/README.md).

Each screenshot has an Impeccable provenance sidecar. The plumbob is the existing `native/assets/plumbob.svg`. Manrope is self-hosted from Google Fonts under the included `public/assets/Manrope-LICENSE.txt` license. No visitor data or keys are stored by the site.

## Search discovery

The English homepage and guide use `/` and `/guide`; Portuguese uses `/pt-br` and `/pt-br/guide`. Each page contains its full text, navigation, descriptive title, social metadata, JSON-LD, and its own canonical URL before JavaScript runs. Reciprocal `hreflang` links identify English, Brazilian Portuguese, and the default English page. Language controls are ordinary links. JavaScript preserves guide sections and preview choices across language navigation and forwards old `?lang=pt-BR` links to the Portuguese pages.

`public/robots.txt` allows crawling and advertises `public/sitemap.xml`, which lists all four canonical pages. Preview query parameters share each language's canonical page. The repository README links to the public website and guide. Keep the repository's About website field set to `https://plumcrate.vercel.app/` so visitors and crawlers can follow it.

These changes follow Google's guidance on [language-specific URLs](https://developers.google.com/search/docs/specialty/international/managing-multi-regional-sites), [crawlable links](https://developers.google.com/search/docs/crawling-indexing/links-crawlable), and [search discovery](https://developers.google.com/search/docs/fundamentals/seo-starter-guide). They do not guarantee indexing or ranking. Google says changes can take weeks or longer to affect results. Useful community references and links from relevant Linux and Sims resources remain ongoing work; do not manufacture links, reviews, or verification tokens.

The homepage also declares the `WebSite` name as PlumCrate, with the domain homepage as its URL in both languages. Google documents this as a [site-name signal](https://developers.google.com/search/docs/appearance/site-names), separate from the existing application metadata. The shared 96-pixel square favicon is rasterized from the existing plumbob SVG, following [Google's favicon guidance](https://developers.google.com/search/docs/appearance/favicon-in-search).

### Notify search engines after publishing

The production build reads `INDEXNOW_KEY` and writes its ownership proof to an unguessable root text-file URL. Keep the same value in the ignored local `website/.env.indexnow.local` file and the Vercel project's sensitive production environment variable. The proof is public by protocol, but the publisher configuration is kept out of git and the key is never embedded in page HTML or logged by the notification script.

After deploying a meaningful content change, run from `website`:

```sh
npm run notify-search
```

The script verifies the deployed ownership proof and submits only this site's canonical sitemap URLs to the [IndexNow endpoint](https://www.indexnow.org/documentation). HTTP 200 means the submission was received; HTTP 202 means receipt with ownership validation pending. Neither confirms crawling, indexing, or ranking. Submit after changes, rather than polling or resubmitting unchanged URLs.

IndexNow shares notifications among its [participating engines](https://www.indexnow.org/faq), which include Bing, a source for DuckDuckGo results. It does not submit a Google indexing request. Google's direct crawl-request route is [URL Inspection in Search Console](https://developers.google.com/search/docs/crawling-indexing/ask-google-to-recrawl), using a verified owner account. Public crawling remains possible without that account workflow.

Run `npm test` to check canonical URLs, structured data, crawl discovery, and the visible product heading alongside existing website tests.
