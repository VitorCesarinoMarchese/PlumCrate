import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const origin='https://ts4-mod-manager.vercel.app';
const read=name=>readFile(new URL(`../public/${name}`,import.meta.url),'utf8');
test('the public repository links to the website and player guide',async()=>{
  const readme=await readFile(new URL('../../README.md',import.meta.url),'utf8');
  assert.ok(readme.includes(`](${origin}/)`),'The repository must link to the production website');
  assert.ok(readme.includes(`](${origin}/guide)`),'The repository must link to the player guide');
});
for(const [file,path] of [['index.html','/'],['guide.html','/guide']]){
  test(`${file} exposes canonical identity and social metadata without JavaScript`,async()=>{
    const html=await read(file);
    assert.ok(html.includes(`<link rel="canonical" href="${origin}${path}">`));
    for(const property of ['og:title','og:description','og:url','og:image']) assert.ok(html.includes(`property="${property}"`),property);
    assert.match(html, /name="twitter:card" content="summary_large_image"/);
    assert.doesNotMatch(html,/noindex/);
    const data=JSON.parse(html.match(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/)[1]);
    assert.equal(data.url,origin+path);
    if(path==='/'){
      assert.equal(data['@type'],'SoftwareApplication');
      assert.equal(data.operatingSystem,'Linux');
      assert.equal(data.offers.price,'0');
      const heading=html.match(/<h1\b[^>]*>([\s\S]*?)<\/h1>/)[1].replace(/<[^>]+>/g,' ');
      assert.match(heading,/TS4 Mod Manager/);
      assert.match(heading,/Linux/);
    }
  });
}
test('crawlers can discover both canonical pages through robots and sitemap',async()=>{
  const robots=await read('robots.txt');
  assert.match(robots,/User-agent: \*\nAllow: \/\n/);
  assert.ok(robots.includes(`Sitemap: ${origin}/sitemap.xml`));
  const sitemap=await read('sitemap.xml');
  assert.ok(sitemap.includes(`xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"`));
  assert.deepEqual([...sitemap.matchAll(/<loc>(.*?)<\/loc>/g)].map(m=>m[1]),[origin+'/',origin+'/guide',origin+'/pt-br',origin+'/pt-br/guide']);
});
