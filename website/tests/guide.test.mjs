import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {navigationItems} from '../public/site-navigation.js';
const publicDir=new URL('../public/',import.meta.url);
test('the visitor guide stays on the website instead of opening the developer reference',async()=>{
  const homepage=await readFile(new URL('index.html',publicDir),'utf8');
  assert.doesNotMatch(homepage,/href="https:\/\/github\.com\/[^\"]*\/docs\/native-app\.md"/);
  assert.match(homepage,/href="\/guide"[^>]*data-i18n="guide"/);
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

test('the guide retains its own destination in the top navigation',async()=>{
  const guide=await readFile(new URL('guide.html',publicDir),'utf8');
  const header=guide.match(/<header>[\s\S]*?<\/header>/)[0];
  assert.match(header,/<a[^>]*data-nav-key="guide"[^>]*aria-current="page"[^>]*>User guide<\/a>/);
});

test('both pages expose the same navigation labels, order, and destinations',async()=>{
  const menus=[];
  for(const file of ['index.html','guide.html']){
    const html=await readFile(new URL(file,publicDir),'utf8');
    const nav=html.match(/<header>[\s\S]*?<nav[^>]*>([\s\S]*?)<\/nav>/)[1];
    menus.push([...nav.matchAll(/<a\b[^>]*href="([^"]+)"[^>]*>([\s\S]*?)<\/a>/g)].map(([,href,text])=>{
      const url=new URL(href,'https://example.com/');
      return {href:url.href,label:text.replace(/<[^>]+>/g,'').replace(/\s+/g,' ').trim()};
    }));
  }
  assert.deepEqual(menus[1],menus[0]);
  assert.deepEqual(menus[0],navigationItems.map(item=>({href:new URL(item.href,'https://example.com/').href,label:item.en})));
});
