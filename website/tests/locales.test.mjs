import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp, readFile, rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import * as navigation from '../public/site-navigation.js';

test('language links preserve the guide section and preview while removing legacy language queries',()=>{
  assert.equal(typeof navigation.languageHref,'function');
  assert.equal(navigation.languageHref('pt-BR','/guide','?lang=en&view=night&previews=1','#import'),'/pt-br/guide?view=night&previews=1#import');
  assert.equal(navigation.languageHref('en','/pt-br/guide','','#trash'),'/guide#trash');
  assert.equal(navigation.languageHref('en','/pt-br/guide.html','','#trash'),'/guide#trash');
  assert.equal(navigation.languageHref('pt-BR','/','?view=studio',''),'/pt-br?view=studio');
  assert.equal(navigation.languageHref('en','/pt-br','',''),'/');
});

test('the deployed artifact exposes complete, linked language pages without JavaScript',async()=>{
  const outputDirectory=await mkdtemp(join(tmpdir(),'ts4-site-'));
  try{
    const {buildSite}=await import('../build.mjs');
    await buildSite({outputDirectory});
    const origin='https://plumcrate.vercel.app';
    const pages=[
      ['index.html','/','en','/pt-br'],
      ['guide.html','/guide','en','/pt-br/guide'],
      ['pt-br.html','/pt-br','pt-BR','/'],
      ['pt-br/guide.html','/pt-br/guide','pt-BR','/guide'],
    ];
    for(const [file,path,language,alternate] of pages){
      const html=await readFile(join(outputDirectory,file),'utf8');
      assert.ok(html.includes(`<html lang="${language}">`),file);
      assert.ok(html.includes(`<link rel="canonical" href="${origin}${path}">`),file);
      for(const code of ['en','pt-BR','x-default']) assert.ok(html.includes(`hreflang="${code}"`),`${file}: ${code}`);
      assert.ok(html.includes(`rel="alternate" hreflang="${language==='en'?'pt-BR':'en'}" href="${origin}${alternate}"`),file);
      const languageMenu=html.match(/<div class="languages"[^>]*>([\s\S]*?)<\/div>/)[1];
      assert.match(languageMenu,/<a\b[^>]*href="[^"]+"[^>]*data-lang="en"/);
      assert.match(languageMenu,/<a\b[^>]*href="[^"]+"[^>]*data-lang="pt-BR"/);
      assert.doesNotMatch(languageMenu,/<button/);
      const data=JSON.parse(html.match(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/)[1]);
      assert.equal(data.url,origin+path);
      assert.equal(data.inLanguage,language);
      if(!path.endsWith('/guide')){
        const blocks=[...html.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)].map(match=>JSON.parse(match[1]));
        const website=blocks.find(item=>item['@type']==='WebSite');
        assert.ok(website,`${file}: website identity is missing`);
        assert.equal(website.name,'PlumCrate');
        assert.equal(website.url,origin+'/',`${file}: website identity must point to the domain homepage`);
      }
      assert.doesNotMatch(html,/noindex|undefined/);
      if(language==='pt-BR'){
        assert.match(html,/<title>[^<]*(?:Gerenciador|Guia)[^<]*<\/title>/);
        assert.match(html,/name="description" content="(?:Gerencie|Aprenda)/);
        assert.ok(html.includes('href="/pt-br/guide"'));
        assert.ok(html.includes('src="/assets/'));
        assert.ok(html.includes('Navegação principal'));
        assert.doesNotMatch(html,/Your mods,<br>step by step|More time playing|Skip to content/);
        assert.match(html,path.endsWith('/guide')?/Escolher a pasta do jogo/:/Gerencie seus mods/);
        if(path.endsWith('/guide')) assert.equal(data.about.url,origin+'/pt-br');
      }
    }
    const sitemap=await readFile(join(outputDirectory,'sitemap.xml'),'utf8');
    assert.deepEqual([...sitemap.matchAll(/<loc>(.*?)<\/loc>/g)].map(m=>m[1]),pages.map(([,path])=>origin+path));
    const first=await readFile(join(outputDirectory,'pt-br/guide.html'),'utf8');
    await buildSite({outputDirectory});
    assert.equal(await readFile(join(outputDirectory,'pt-br/guide.html'),'utf8'),first,'Builds must be repeatable');
  }finally{await rm(outputDirectory,{recursive:true,force:true});}
});

test('the guide keeps its descriptive search title after JavaScript runs',async()=>{
  const script=await readFile(new URL('../public/guide.js',import.meta.url),'utf8');
  assert.doesNotMatch(script,/document\.title\s*=/);
});
