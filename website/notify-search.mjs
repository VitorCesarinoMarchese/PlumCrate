import {readFile} from 'node:fs/promises';
import {indexNowSubmission} from './search-discovery.mjs';

const sitemap = await readFile(new URL('./public/sitemap.xml', import.meta.url), 'utf8');
const submission = indexNowSubmission(sitemap, process.env.INDEXNOW_KEY);
const proof = await fetch(submission.keyLocation, {signal: AbortSignal.timeout(20000), redirect: 'error'});
if (!proof.ok || (await proof.text()).trim() !== submission.key) {
  throw new Error('Deploy the configured IndexNow ownership proof before submitting URLs');
}
const response = await fetch('https://api.indexnow.org/indexnow', {
  method: 'POST',
  headers: {'Content-Type': 'application/json; charset=utf-8'},
  body: JSON.stringify(submission),
  signal: AbortSignal.timeout(20000),
});
if (![200, 202].includes(response.status)) throw new Error(`IndexNow submission failed: HTTP ${response.status}`);
console.log(JSON.stringify({status: response.status, submittedUrls: submission.urlList.length,
  result: response.status === 202 ? 'Received; ownership validation pending' : 'Received',
  indexingGuaranteed: false}));
