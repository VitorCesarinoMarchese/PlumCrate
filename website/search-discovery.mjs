export const siteOrigin = 'https://plumcrate.vercel.app';

export function indexNowKeyFilename(key) {
  if (typeof key !== 'string' || !/^[a-zA-Z0-9-]{8,128}$/.test(key)) {
    throw new Error('A valid IndexNow ownership key is required');
  }
  return `${key}.txt`;
}

export function indexNowSubmission(sitemap, key) {
  const filename = indexNowKeyFilename(key);
  const urlList = [...new Set([...sitemap.matchAll(/<loc>(.*?)<\/loc>/g)].map(match => match[1]))];
  if (!urlList.length || urlList.length > 10000 || urlList.some(raw => {
    const url = new URL(raw);
    return url.origin !== siteOrigin || url.search || url.hash;
  })) throw new Error('Submit only this site’s canonical sitemap URLs');
  return {host: new URL(siteOrigin).host, key, keyLocation: `${siteOrigin}/${filename}`, urlList};
}
