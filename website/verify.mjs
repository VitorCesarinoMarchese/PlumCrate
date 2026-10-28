import assert from 'node:assert/strict';
import {once} from 'node:events';
import {mkdir} from 'node:fs/promises';
import {chromium} from 'playwright';
import {buildSite} from './build.mjs';
import {createPreviewServer} from './dev.mjs';

await buildSite();
const server = createPreviewServer().listen(0, '127.0.0.1');
await once(server, 'listening');
const origin = `http://127.0.0.1:${server.address().port}`;
let browser;
let checked = 0;
try {
  browser = await chromium.launch();
  for (const javaScriptEnabled of [false, true]) {
    for (const width of [1440, 390]) {
      const context = await browser.newContext({javaScriptEnabled, viewport: {width, height: 900}});
      await context.route('**/api/release', route => route.fulfill({json: {
        available: false, url: 'https://github.com/VitorCesarinoMarchese/PlumCrate/releases',
      }}));
      const page = await context.newPage();
      const errors = [];
      page.on('pageerror', error => errors.push(error.message));
      for (const path of ['/', '/guide', '/pt-br', '/pt-br/guide']) {
        const response = await page.goto(origin + path, {waitUntil: 'networkidle'});
        assert.equal(response.status(), 200, path);
        const language = path.startsWith('/pt-br') ? 'pt-BR' : 'en';
        assert.equal(await page.locator('html').getAttribute('lang'), language);
        assert.match(await page.title(), /PlumCrate.*Linux|PlumCrate.*The Sims 4/);
        assert.equal(await page.locator('link[rel="canonical"]').getAttribute('href'), `https://plumcrate.vercel.app${path}`);
        assert.equal(await page.locator('.languages a').count(), 2);
        assert.equal(await page.locator('.languages [aria-current="true"]').getAttribute('data-lang'), language);
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, `${path} overflows at ${width}px`);
        assert.equal(await page.locator('main img').first().evaluate(image => image.complete && image.naturalWidth > 0), true, `${path} image`);
        const alternate = language === 'en' ? 'pt-BR' : 'en';
        await page.locator(`.languages a[data-lang="${alternate}"]`).click();
        assert.equal(await page.locator('html').getAttribute('lang'), alternate);
        if (javaScriptEnabled) {
          await page.goto(origin + path, {waitUntil: 'networkidle'});
          const directory = new URL('../.impeccable/review/google-seo/', import.meta.url);
          await mkdir(directory, {recursive: true});
          const name = `${path.replaceAll('/', '-') || 'home'}-${width}.png`;
          await page.screenshot({path: new URL(name, directory).pathname});
        }
        checked++;
      }
      if (javaScriptEnabled) {
        await page.goto(`${origin}/guide?lang=pt-BR#import`);
        await page.waitForURL('**/pt-br/guide#import');
        await page.locator('.languages a[data-lang="en"]').click();
        await page.waitForURL('**/guide#import');
        await page.goto(`${origin}/?previews=1&view=night`);
        await page.locator('.languages a[data-lang="pt-BR"]').click();
        await page.waitForURL('**/pt-br?previews=1&view=night');
        assert.equal(await page.locator('html').getAttribute('data-view'), 'night');
        await page.locator('[data-screen="settings"]').click();
        assert.equal(await page.locator('#app-image').getAttribute('src'), '/assets/settings.webp');
        assert.match(await page.locator('#app-image').getAttribute('alt'), /Configurações/);
      }
      assert.deepEqual(errors, [], 'No browser JavaScript errors');
      await context.close();
    }
  }
  console.log(`Verified ${checked} page/language/viewport combinations, language navigation, legacy links, preview state, and screenshot controls.`);
} finally {
  await browser?.close();
  await new Promise(resolve => server.close(resolve));
}
