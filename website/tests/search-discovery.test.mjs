import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';

const key='0123456789abcdef0123456789abcdef';
const origin='https://plumcrate.vercel.app';

test('crawl notifications submit only canonical pages from this site',async()=>{
  const {indexNowSubmission}=await import('../search-discovery.mjs');
  const sitemap=await readFile(new URL('../public/sitemap.xml',import.meta.url),'utf8');
  const data=indexNowSubmission(sitemap,key);
  assert.equal(data.host,'plumcrate.vercel.app');
  assert.equal(data.key,key);
  assert.equal(data.keyLocation,`${origin}/${key}.txt`);
  assert.deepEqual(data.urlList,['/','/guide','/pt-br','/pt-br/guide'].map(path=>origin+path));
  assert.throws(()=>indexNowSubmission(sitemap.replace(origin,'https://other.example'),key),/canonical/);
  assert.throws(()=>indexNowSubmission('<urlset/>',key),/canonical/);
  assert.throws(()=>indexNowSubmission(sitemap,'../unsafe'),/key/);
});

test('production builds publish the configured ownership proof without embedding it in page HTML',async()=>{
  const {buildSite}=await import('../build.mjs');
  const outputDirectory=await mkdtemp(join(tmpdir(),'plumcrate-discovery-'));
  try{
    await buildSite({outputDirectory,indexNowKey:key});
    assert.equal((await readFile(join(outputDirectory,`${key}.txt`),'utf8')).trim(),key);
    assert.ok(!(await readFile(join(outputDirectory,'index.html'),'utf8')).includes(key));
  }finally{await rm(outputDirectory,{recursive:true,force:true});}
});
