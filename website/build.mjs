import {cp, mkdir, readFile, writeFile} from 'node:fs/promises';
import {dirname, join} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {parse, parseFragment, serialize} from 'parse5';
import {homepage, guide} from './copy/pt-BR.mjs';
import {languageHref, navigationItems} from './public/site-navigation.js';
import {siteOrigin as origin, indexNowKeyFilename} from './search-discovery.mjs';

const publicDirectory = fileURLToPath(new URL('./public/', import.meta.url));
export const defaultOutputDirectory = fileURLToPath(new URL('./dist/', import.meta.url));
const pages = [
  {file: 'index.html', path: '/', localizedFile: 'pt-br.html', copy: homepage,
    title: 'PlumCrate para Linux | Gerenciador de Mods do The Sims 4',
    description: 'Gerencie seus mods e conteúdo personalizado do The Sims 4 no Linux. Importe pastas ou ZIPs, organize sua biblioteca e ative ou desative mods. Grátis e de código aberto.'},
  {file: 'guide.html', path: '/guide', localizedFile: 'pt-br/guide.html', copy: guide,
    title: 'Guia do PlumCrate | Mods do The Sims 4 no Linux',
    description: 'Aprenda a instalar o PlumCrate no Linux, escolher a pasta do jogo, importar mods e conteúdo personalizado, ativar mods e restaurar arquivos.'},
];

function* elements(node) {
  if (node.tagName) yield node;
  for (const child of node.childNodes || []) yield* elements(child);
}
const attribute = (node, name) => node.attrs.find(attr => attr.name === name)?.value;
function setAttribute(node, name, value) {
  const attr = node.attrs.find(attr => attr.name === name);
  if (attr) attr.value = value;
  else node.attrs.push({name, value});
}
function content(node, html) {
  node.childNodes = parseFragment(html).childNodes;
  for (const child of node.childNodes) child.parentNode = node;
}

function renderPage(html, page, language) {
  const document = parse(html);
  const nodes = [...elements(document)];
  const localizedPath = languageHref(language, page.path);
  const labels = {'Main navigation': 'Navegação principal', Language: 'Idioma',
    'App screenshots': 'Capturas do aplicativo', 'Website previews': 'Prévias do site', 'Guide contents': 'Conteúdo do guia'};
  for (const node of nodes) {
    if (language === 'pt-BR') {
      const key = attribute(node, 'data-i18n') ?? attribute(node, 'data-guide-i18n');
      if (key) {
        if (!(key in page.copy)) throw new Error(`Missing Portuguese copy: ${page.file} ${key}`);
        content(node, page.copy[key]);
      }
      const nav = navigationItems.find(item => item.key === attribute(node, 'data-nav-key'));
      if (nav) content(node, nav.pt);
      const label = attribute(node, 'aria-label');
      if (label in labels) setAttribute(node, 'aria-label', labels[label]);
      if (attribute(node, 'class') === 'skip') content(node, 'Pular para o conteúdo');
      if (node.tagName === 'title') content(node, page.title);
      const name = attribute(node, 'name') ?? attribute(node, 'property');
      if (name === 'description' || name === 'og:description') setAttribute(node, 'content', page.description);
      if (name === 'og:title') setAttribute(node, 'content', page.title);
      if (name === 'og:url') setAttribute(node, 'content', origin + localizedPath);
      if (name === 'og:image:alt') setAttribute(node, 'content', 'Biblioteca do PlumCrate no Linux com capas e interruptores de mods');
      if (node.tagName === 'img' && attribute(node, 'alt')) setAttribute(node, 'alt', 'Biblioteca real do PlumCrate no tema claro, com capas dos mods e interruptores');
    }
    if (node.tagName === 'html') setAttribute(node, 'lang', language);
    if (attribute(node, 'rel') === 'canonical') setAttribute(node, 'href', origin + localizedPath);
    if (attribute(node, 'type') === 'application/ld+json') {
      const data = JSON.parse(node.childNodes.map(child => child.value || '').join(''));
      const isWebsite = data['@type'] === 'WebSite';
      data.url = origin + (isWebsite ? '/' : localizedPath);
      data.inLanguage = isWebsite ? ['en', 'pt-BR'] : language;
      if (language === 'pt-BR' && !isWebsite) {
        data.description = page.description;
        if (data['@type'] === 'TechArticle') data.name = 'Guia do PlumCrate';
        if (data.about) data.about.url = origin + '/pt-br';
      }
      node.childNodes[0].value = JSON.stringify(data, null, 2);
    }
  }
  // Include links inside translated fragments, which were added after the first traversal.
  for (const node of elements(document)) {
    const href = attribute(node, 'href');
    if (href && !/^(?:https?:|#|\?)/.test(href)) {
      const url = new URL(href, origin);
      const path = url.pathname.replace(/\.html$/, '');
      setAttribute(node, 'href', path === '/' || path === '/guide'
        ? languageHref(language, path, url.search, url.hash) : href);
    }
    const linkLanguage = attribute(node, 'data-lang');
    if (linkLanguage) {
      setAttribute(node, 'href', languageHref(linkLanguage, page.path));
      node.attrs = node.attrs.filter(attr => attr.name !== 'aria-current');
      if (linkLanguage === language) setAttribute(node, 'aria-current', 'true');
    }
  }
  const head = nodes.find(node => node.tagName === 'head');
  const alternates = ['en', 'pt-BR', 'x-default'].map(code =>
    `<link rel="alternate" hreflang="${code}" href="${origin}${languageHref(code, page.path)}">`).join('\n');
  for (const link of parseFragment(alternates).childNodes) {
    link.parentNode = head;
    head.childNodes.push(link);
  }
  return serialize(document);
}

export async function buildSite({outputDirectory = defaultOutputDirectory, indexNowKey = process.env.INDEXNOW_KEY} = {}) {
  await cp(publicDirectory, outputDirectory, {recursive: true});
  for (const page of pages) {
    const html = await readFile(join(publicDirectory, page.file), 'utf8');
    for (const [language, file] of [['en', page.file], ['pt-BR', page.localizedFile]]) {
      const destination = join(outputDirectory, file);
      await mkdir(dirname(destination), {recursive: true});
      await writeFile(destination, renderPage(html, page, language));
    }
  }
  if (indexNowKey !== undefined) {
    await writeFile(join(outputDirectory, indexNowKeyFilename(indexNowKey)), indexNowKey + '\n');
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await buildSite();
