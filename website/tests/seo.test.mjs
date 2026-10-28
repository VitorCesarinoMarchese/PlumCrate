import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const origin='https://plumcrate.vercel.app';
const read=name=>readFile(new URL(`../public/${name}`,import.meta.url),'utf8');
test('the homepage declares PlumCrate as the website name for search engines',async()=>{
  const html=await read('index.html');
  const data=[...html.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)].map(match=>JSON.parse(match[1]));
  const website=data.find(item=>item['@type']==='WebSite');
  assert.ok(website,'Google site-name markup is missing');
  assert.equal(website.name,'PlumCrate');
  assert.equal(website.url,origin+'/');
});
test('both pages use the same square favicon suitable for search results',async()=>{
  for(const file of ['index.html','guide.html']){
    assert.match(await read(file),/<link rel="icon" href="\/favicon\.png" sizes="96x96" type="image\/png">/);
  }
  const png=await readFile(new URL('../public/favicon.png',import.meta.url));
  assert.equal(png.subarray(0,8).toString('hex'),'89504e470d0a1a0a');
  assert.equal(png.readUInt32BE(16),96);
  assert.equal(png.readUInt32BE(20),96);
});
test('old website links redirect permanently to the corresponding PlumCrate page',async()=>{
  const config=JSON.parse(await readFile(new URL('../vercel.json',import.meta.url),'utf8'));
  const redirect=config.redirects?.find(item=>item.source==='/:path*'&&item.has?.some(condition=>condition.type==='host'&&condition.value==='ts4-mod-manager.vercel.app'));
  assert.ok(redirect,'The old public host needs a redirect');
  assert.equal(redirect.source,'/:path*');
  assert.equal(redirect.destination,`${origin}/:path*`);
  assert.equal(redirect.permanent,true);
  const homepage=config.redirects.find(item=>item.source==='/'&&item.has?.some(condition=>condition.type==='host'&&condition.value==='ts4-mod-manager.vercel.app'));
  assert.ok(homepage,'The bare homepage needs its own permanent redirect');
  assert.equal(homepage.destination,`${origin}/`);
  assert.equal(homepage.permanent,true);
});
test('the public app identity is PlumCrate, with Sims 4 as its description',async()=>{
  for(const file of ['index.html','guide.html']){
    const html=await read(file);
    assert.match(html,/<title>PlumCrate/);
    assert.match(html,/<meta property="og:site_name" content="PlumCrate">/);
    assert.match(html,/<strong>PlumCrate<\/strong>/);
    assert.doesNotMatch(html,/TS4 Mod Manager|Sims 4 Mod Manager/);
  }
});
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
      assert.match(heading,/PlumCrate/);
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
