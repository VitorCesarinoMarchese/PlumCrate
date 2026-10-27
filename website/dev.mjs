import http from 'node:http';
import {readFile} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';
import path from 'node:path';
import handler from './api/release.js';
import {buildSite, defaultOutputDirectory} from './build.mjs';
const mime={'.html':'text/html','.css':'text/css','.js':'text/javascript','.png':'image/png','.webp':'image/webp','.svg':'image/svg+xml','.ttf':'font/ttf','.txt':'text/plain','.xml':'application/xml'};
export function createPreviewServer() {
  return http.createServer(async(req,res)=>{
    let pathname;
    try {pathname=decodeURIComponent(new URL(req.url,'http://localhost').pathname)}
    catch {res.writeHead(400);return res.end('Invalid URL')}
    if(pathname==='/api/release') {res.status=n=>{res.statusCode=n;return res};res.json=data=>res.end(JSON.stringify(data));res.setHeader('Content-Type','application/json');return handler(req,res)}
    const route=pathname==='/'?'/index.html':pathname.replace(/\/$/,'');
    const file=path.resolve(defaultOutputDirectory,'.'+(path.extname(route)?route:route+'.html'));
    if(path.relative(defaultOutputDirectory,file).startsWith('..')){res.writeHead(403);return res.end()}
    try {const body=await readFile(file);res.setHeader('Content-Type',mime[path.extname(file)]||'application/octet-stream');res.end(body)}catch{res.writeHead(404);res.end('Not found')}
  });
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await buildSite();
  createPreviewServer().listen(4173,'127.0.0.1',()=>console.log('Website: http://localhost:4173'));
}
