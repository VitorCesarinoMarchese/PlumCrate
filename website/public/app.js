import {initializeNavigation} from './site-navigation.js';

const views=['gallery','home','night','guide','studio'];
const params=new URLSearchParams(location.search);
const language=document.documentElement.lang;
initializeNavigation();
let currentView=views.includes(params.get('view'))?params.get('view'):'gallery';
let release;
function syncUrl(){if(params.has('view')||currentView!=='gallery')params.set('view',currentView);params.delete('lang');const query=params.size?`?${params}`:'';history.replaceState({},'',`${location.pathname}${query}${location.hash}`);const clean=new URLSearchParams(params);clean.set('clean','1');document.querySelector('#clean-link').href=`?${clean}`;initializeNavigation();}
function renderRelease(){if(!release)return;const isPt=language==='pt-BR';const button=document.querySelector('.download');button.href=release.url;button.querySelector('span').textContent=release.available?(isPt?'Baixar para Linux':'Download for Linux'):(isPt?'Ver versões no GitHub':'View GitHub releases');document.querySelector('.release-status').textContent=release.available?`${release.version} · ${release.name}${release.prerelease?(isPt?' · Pré-lançamento':' · Prerelease'):''}`:release.error?(isPt?'Não foi possível consultar o GitHub. Veja as versões disponíveis no link acima.':'Could not check GitHub. Use the link above to browse available releases.'):(isPt?'Nenhum AppImage publicado ainda. Você pode compilar o código seguindo o guia no GitHub.':'No AppImage published yet. You can build from source using the GitHub guide.');}
let screen='light';
function updateAlt(){document.querySelector('#app-image').alt=language==='pt-BR'?(screen==='settings'?'Configurações reais do aplicativo com opções de aparência':`Biblioteca real do Sims 4 Mod Manager no tema ${screen==='dark'?'escuro':'claro'}, com capas dos mods e interruptores`):(screen==='settings'?'Real app settings with appearance controls':`Real Sims 4 Mod Manager ${screen} library with mod covers and on/off switches`);}
function setScreen(value){screen=value;const img=document.querySelector('#app-image');const src=`/assets/${value==='settings'?'settings':'library-'+value}.webp`;img.src=src;document.querySelector('#image-link').href=src;img.classList.remove('image-swap');void img.offsetWidth;img.classList.add('image-swap');document.querySelectorAll('[data-screen]').forEach(b=>b.setAttribute('aria-pressed',b.dataset.screen===value));updateAlt();}
function setView(view){currentView=view;document.documentElement.dataset.view=view;document.querySelectorAll('[data-view-button]').forEach(b=>b.setAttribute('aria-pressed',b.dataset.viewButton===view));setScreen(view==='night'?'dark':'light');syncUrl();}
for(const [i,view] of views.entries()){const button=document.createElement('button');button.dataset.viewButton=view;button.textContent=`${i+1} ${view[0].toUpperCase()+view.slice(1)}`;button.addEventListener('click',()=>setView(view));document.querySelector('#variants').append(button);}
document.querySelectorAll('[data-screen]').forEach(b=>b.addEventListener('click',()=>setScreen(b.dataset.screen)));
if(params.get('previews')==='1'&&params.get('clean')!=='1')document.body.classList.remove('clean');
setView(currentView);
fetch('/api/release').then(async response=>{const data=await response.json();if(typeof data.available!=='boolean'||!data.url?.startsWith('https://github.com/VitorCesarinoMarchese/TS4ModManager/releases'))throw new Error('Invalid release');release=data;renderRelease();}).catch(()=>{release={available:false,error:true,url:'https://github.com/VitorCesarinoMarchese/TS4ModManager/releases'};renderRelease()});
