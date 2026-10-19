import http from 'node:http';
import {readFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import handler from './api/release.js';
const root=fileURLToPath(new URL('./public/',import.meta.url));
const mime={'.html':'text/html','.css':'text/css','.js':'text/javascript','.png':'image/png','.webp':'image/webp','.svg':'image/svg+xml','.woff2':'font/woff2'};
http.createServer(async(req,res)=>{
  const pathname=decodeURIComponent(new URL(req.url,'http://localhost').pathname);
  if(pathname==='/api/release') {res.status=n=>{res.statusCode=n;return res};res.json=data=>res.end(JSON.stringify(data));res.setHeader('Content-Type','application/json');return handler(req,res)}
  const file=path.resolve(root,'.'+(pathname==='/'?'/index.html':pathname));
  if(!file.startsWith(root)){res.writeHead(403);return res.end()}
  try {const body=await readFile(file);res.setHeader('Content-Type',mime[path.extname(file)]||'application/octet-stream');res.end(body)}catch{res.writeHead(404);res.end('Not found')}
}).listen(4173,'0.0.0.0',()=>console.log('Website previews: http://localhost:4173'));
