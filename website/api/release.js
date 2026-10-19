const repo = 'VitorCesarinoMarchese/TS4ModManager';
const releasesUrl = `https://github.com/${repo}/releases`;
export function selectRelease(releases) {
  for (const release of [...releases].filter(r => !r.draft && r.published_at).sort((a,b)=>Date.parse(b.published_at)-Date.parse(a.published_at))) {
    const asset = release.assets?.find(a => /\.AppImage$/i.test(a.name) && !/arm|aarch64|i686/i.test(a.name) && a.browser_download_url?.startsWith(`https://github.com/${repo}/releases/download/`));
    if (asset) return {available:true,version:release.tag_name,url:asset.browser_download_url,name:asset.name,size:asset.size,prerelease:!!release.prerelease};
  }
  return {available:false,url:releasesUrl};
}
export default async function handler(_req,res) {
  res.setHeader('Cache-Control','public, s-maxage=300, stale-while-revalidate=3600');
  try {
    const response = await fetch(`https://api.github.com/repos/${repo}/releases?per_page=30`, {headers:{Accept:'application/vnd.github+json','User-Agent':'TS4ModManager-website'},signal:AbortSignal.timeout(8000)});
    if (!response.ok) throw new Error('GitHub release lookup failed');
    return res.status(200).json(selectRelease(await response.json()));
  } catch {
    res.setHeader('Cache-Control','no-store');
    return res.status(503).json({available:false,error:true,url:releasesUrl});
  }
}
