import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const publicDir=new URL('../public/',import.meta.url);
test('the visitor guide stays on the website instead of opening the developer reference',async()=>{
  const homepage=await readFile(new URL('index.html',publicDir),'utf8');
  assert.doesNotMatch(homepage,/href="https:\/\/github\.com\/[^\"]*\/docs\/native-app\.md"/);
  assert.match(homepage,/href="guide(?:\.html)?"[^>]*data-i18n="guide"/);
});
test('the user guide has valid task navigation and official CurseForge sources',async()=>{
  const guide=await readFile(new URL('guide.html',publicDir),'utf8');
  const ids=new Set([...guide.matchAll(/\bid="([^"]+)"/g)].map(m=>m[1]));
  for(const match of guide.matchAll(/href="#([^"]+)"/g)) assert.ok(ids.has(match[1]),`Missing section ${match[1]}`);
  for(const id of ['install','game-folder','import','library','curseforge','updates','trash','settings','help']) assert.ok(ids.has(id),`Missing task ${id}`);
  assert.match(guide,/9000208346-about-the-curseforge-api-and-how-to-apply-for-a-key/);
  assert.match(guide,/9000207405-curse-forge-3rd-party-api-terms-and-conditions/);
  assert.doesNotMatch(guide,/<input\b/i,'The website must never collect an API key');
});
