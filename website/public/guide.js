import {currentSectionIndex} from './guide-navigation.js';

const pt={
  guide:'Guia de uso',skip:'Pular para o conteúdo',home:'O aplicativo',downloads:'Downloads',support:'Ajuda ↗',contents:'Neste guia',
  navInstall:'Baixar o aplicativo',navFolder:'Escolher a pasta do jogo',navImport:'Importar seus mods',navLibrary:'Usar sua biblioteca',navCurseforge:'CurseForge e acesso à API',navUpdates:'Verificar atualizações',navTrash:'Remover e restaurar',navSettings:'Salvar configurações',navHelp:'Resolver problemas',
  title:'Seus mods,<br>passo a passo.',
  intro:'Um guia para usar o Sims 4 Mod Manager no Linux. Comece com os mods que você já tem. Você não precisa de uma chave de API para gerenciar mods locais.',
  alpha:'O aplicativo está em alpha inicial. Faça um backup separado da sua coleção e dos jogos salvos antes de usá-lo com seu jogo principal.',
  installTitle:'Baixar o aplicativo',
  installBody:`<ol><li>Abra as <a href="https://github.com/VitorCesarinoMarchese/TS4ModManager/releases">versões no GitHub</a> e escolha o AppImage para Linux quando estiver disponível. Se não houver um AppImage, ainda não há um download pronto para usar.</li><li>Salve o AppImage em uma pasta onde deseja mantê-lo. No gerenciador de arquivos, abra suas propriedades e permita executá-lo como programa. O nome da opção varia conforme o ambiente Linux.</li><li>Abra o AppImage para iniciar o aplicativo.</li></ol><p>Feche o The Sims 4 antes de alterar sua coleção de mods.</p>`,
  folderTitle:'Escolher a pasta do jogo',
  folderBody:`<ol><li>Abra o controle de pasta no topo do aplicativo. Use <strong>Choose folder</strong> ou <strong>Change folder</strong> se o local detectado estiver errado.</li><li>Selecione a pasta de dados <strong>The Sims 4</strong> que contém <strong>Mods</strong>. Selecione a pasta acima de Mods, não a própria Mods nem a pasta de instalação do jogo.</li><li>Confira se o aplicativo está usando a pasta do jogo que você realmente joga. Instalações pelo Wine ou Proton no Linux podem ter mais de um local de dados.</li></ol><p>Inicie o jogo uma vez se ele ainda não criou suas pastas de dados. Você pode mudar a pasta selecionada depois.</p><h3>Permitir que o jogo carregue os mods</h3><p>No The Sims 4, abra <strong>Opções de Jogo → Outros</strong>. Ative <strong>Habilitar Conteúdo Personalizado e Modificações</strong> e, se seus mods exigirem, <strong>Modificações de Script Permitidas</strong>. Aplique as alterações e reinicie o jogo. Atualizações do jogo podem desativar os mods novamente. Siga as instruções de compatibilidade do criador antes de reativá-los.</p><p class="guide-source">Configurações do jogo: <a href="https://help.ea.com/en/articles/the-sims/the-sims-4/how-to-install-mods-the-sims-4/">guia de instalação de mods da EA</a>.</p>`,
  importTitle:'Importar seus mods',
  importBody:`<ol><li>Em <strong>Your mods</strong>, abra <strong>Add mods</strong>.</li><li>Escolha <strong>Import folder</strong> para uma pasta de mod ou <strong>Import ZIP</strong> para um arquivo compactado baixado.</li><li>Selecione o arquivo ou pasta. Revise o caminho e o nome sugerido e selecione <strong>Import</strong>.</li></ol><p>O aplicativo copia o mod para a biblioteca gerenciada e o ativa no jogo selecionado. O download original continua no lugar. Se a ativação for bloqueada, o mod importado permanece na biblioteca. Leia o status na parte inferior, resolva o conflito e ative essa entrada em vez de importar novamente.</p><p>Já tem mods na pasta do jogo? Revise os arquivos existentes antes de concordar em movê-los para a biblioteca gerenciada.</p>`,
  libraryTitle:'Usar sua biblioteca',
  libraryBody:`<p>Busque nomes e arquivos enquanto digita. Use <strong>All</strong>, <strong>On</strong> e <strong>Off</strong> para filtrar os resultados. Os controles de cartões e lista alternam entre fotos e linhas compactas.</p><p>Use o <strong>interruptor On/Off</strong> de um mod para escolher se ele carrega no jogo selecionado. A alteração é imediata. Selecione o cartão ou linha para inspecionar arquivos e informações da fonte. Clique com o botão direito em uma entrada para ver mais ações.</p><p><strong>On</strong> significa que o aplicativo ativou os arquivos. Isso não garante que o mod seja compatível com a versão do jogo ou que todas as dependências estejam instaladas. Leia as instruções do criador.</p>`,
  caption:'Captura real do aplicativo com uma biblioteca de demonstração e arte real dos criadores. <a href="https://github.com/VitorCesarinoMarchese/TS4ModManager/blob/main/docs/images/README.md">Créditos das imagens ↗</a>',
  curseforgeTitle:'CurseForge e acesso à API',
  curseforgeBody:`<p>Explorar o CurseForge, buscar fontes, instalar pelo navegador e verificar atualizações exige uma chave de API autorizada. Importações e gerenciamento da biblioteca local funcionam sem ela. Você pode baixar mods na página oficial do criador pelo navegador e importá-los localmente.</p><h3>Posso obter uma chave pessoal?</h3><p>O CurseForge documenta um processo de solicitação para desenvolvedores de terceiros, seguido de análise pela Overwolf. Ele não confirma um processo geral de chaves pessoais para jogadores que usam este aplicativo. Os termos restringem a transferência e o compartilhamento de chaves. Não podemos garantir que o uso pretendido seja aprovado.</p><p>Se precisar de acesso à API, leia as <a href="https://support.curseforge.com/support/solutions/articles/9000208346-about-the-curseforge-api-and-how-to-apply-for-a-key">instruções oficiais de solicitação</a> e os <a href="https://support.curseforge.com/support/solutions/articles/9000207405-curse-forge-3rd-party-api-terms-and-conditions">termos da API, seção 2.2</a>. Siga o link de solicitação no artigo, descreva seu uso com precisão e aguarde uma decisão. Consulte o <a href="https://support.curseforge.com/support/tickets/new">suporte do CurseForge</a> se não souber se seu uso está coberto. Este guia não concede permissão para usar uma chave de API.</p><h3>Já tem acesso autorizado?</h3><ol><li>No aplicativo de desktop, abra <strong>Settings → Connections</strong>.</li><li>Insira sua chave autorizada em <strong>CurseForge API key</strong>.</li><li>Selecione <strong>Save changes</strong> e volte para <strong>Discover</strong>.</li></ol><p>Insira a chave apenas no aplicativo local de desktop. Não a cole neste site, em conversas, em issues ou em capturas de tela. Configurações não salvas não alteram a chave usada nas requisições.</p><h3>Instalar um mod pelo Discover</h3><ol><li>Busque um projeto e abra <strong>View details</strong>. Leia a descrição e as dependências.</li><li>Escolha <strong>Download via website…</strong>. Revise a versão, a pasta de Downloads do navegador e a pasta de destino do The Sims 4.</li><li>Selecione <strong>Start browser download</strong>. Conclua qualquer ação de Download ou salvamento na página oficial.</li><li>Mantenha o aplicativo aberto. Ele confere o download concluído, importa o mod e o ativa no jogo que você aprovou. O download original continua no lugar.</li></ol><p>Revise cada dependência separadamente. Se nenhuma versão adequada for oferecida, use o site oficial e a importação local.</p><p class="guide-source">Informações oficiais da API verificadas em 20 de outubro de 2026. O CurseForge decide o acesso e pode alterar seus requisitos.</p>`,
  updatesTitle:'Verificar atualizações de mods',
  updatesBody:`<p>Selecione <strong>Check updates</strong> em <strong>Your mods</strong> ou <strong>Updates</strong>. A verificação exige acesso autorizado ao CurseForge e um projeto do CurseForge confirmado e vinculado ao mod. Um link de site salvo manualmente não é suficiente.</p><ol><li>Se necessário, use <strong>Find source</strong> e confirme o projeto correto. Revise as evidências. Um nome parecido não comprova a correspondência.</li><li>Se a versão instalada for desconhecida, abra os detalhes do mod. Em <strong>Record installed release</strong>, selecione a versão que você realmente instalou, revise e confirme. Se não conseguir identificá-la, deixe a versão como desconhecida.</li><li>Selecione <strong>Check this mod</strong>. Se houver uma atualização, use <strong>Open mod page</strong>, leia as instruções da versão e faça o download.</li></ol><p>A verificação não substitui arquivos dos mods. Você substitui os arquivos locais e depois registra a nova versão instalada. Uma versão mais recente pode ser destinada a outra versão do jogo. Importações assistidas pelo navegador já registram a versão instalada quando você aprova a instalação.</p>`,
  trashTitle:'Remover e restaurar',
  trashBody:`<p>Para manter um mod sem carregá-lo, deixe-o em <strong>Off</strong>. Para remover um mod gerenciado, abra suas ações, escolha <strong>Move to trash</strong> e revise a confirmação.</p><p>Abra <strong>Trash</strong> ou <strong>Restore from trash</strong> para encontrar entradas recuperáveis. Escolha a entrada, revise o destino e confirme a restauração. O aplicativo verifica se os arquivos pertencem ao gerenciamento dele antes de remover seus links e preserva arquivos não gerenciados.</p>`,
  settingsTitle:'Salvar suas configurações',
  settingsBody:`<p><strong>Appearance</strong> altera os temas, <strong>Folders</strong> gerencia pastas de jogos lembradas e <strong>Connections</strong> contém a chave de API opcional. Selecione <strong>Save changes</strong> para aplicar suas alterações. Sair das Configurações mantém o rascunho; <strong>Discard changes</strong> restaura os valores salvos.</p><p><strong>Backup</strong> exporta configurações, não seus arquivos de mods nem jogos salvos. Chaves de API são excluídas, a menos que você as inclua explicitamente. Use um backup separado para sua coleção e jogos salvos.</p><p>Quem usa o AppImage pode verificar <strong>App updates</strong> para encontrar uma nova versão do aplicativo. Atualizações do app e de mods são diferentes. Baixar ou reiniciar para uma atualização do aplicativo exige sua aprovação.</p>`,
  helpTitle:'Quando algo der errado',
  helpBody:`<dl><dt>Um mod está On, mas não aparece no jogo</dt><dd>Confira a pasta de dados selecionada, as opções de mods do jogo, a versão do jogo suportada pelo mod e as dependências do criador. Reinicie o jogo depois de mudar suas opções de mods.</dd><dt>A importação terminou, mas não conseguiu ativar</dt><dd>Leia o status na parte inferior para identificar o conflito. A cópia importada continua na biblioteca. Resolva o conflito antes de ativá-la novamente.</dd><dt>O Discover rejeita minha chave</dt><dd>Confira se você selecionou Save changes e se a chave está autorizada para esse uso. Consulte o CurseForge sobre problemas de acesso. Você ainda pode gerenciar mods locais sem uma chave.</dd><dt>Preciso reportar um bug</dt><dd><a href="https://github.com/VitorCesarinoMarchese/TS4ModManager/issues">Abra uma issue</a> com sua distribuição Linux, versão do app, os passos realizados e a mensagem exibida. Oculte chaves de API e caminhos privados nas capturas de tela.</dd></dl>`,
  back:'Voltar para o site do aplicativo',independent:'Projeto independente. Sem afiliação com a Electronic Arts ou Maxis.'
};
const en=Object.fromEntries([...document.querySelectorAll('[data-guide-i18n]')].map(el=>[el.dataset.guideI18n,el.innerHTML]));
const params=new URLSearchParams(location.search);
function setLanguage(lang){
  const copy=lang==='pt-BR'?pt:en;
  document.documentElement.lang=lang;
  document.title=lang==='pt-BR'?'Guia de uso · Sims 4 Mod Manager':'User guide · Sims 4 Mod Manager';
  document.querySelectorAll('[data-guide-i18n]').forEach(el=>{el.innerHTML=copy[el.dataset.guideI18n]});
  document.querySelectorAll('[data-lang]').forEach(button=>button.setAttribute('aria-pressed',button.dataset.lang===lang));
  document.querySelectorAll('[data-home-link]').forEach(link=>{link.href=`./?lang=${lang}`});
  document.querySelector('[data-guide-page-link]').href=`guide.html?lang=${lang}`;
  document.querySelector('header nav').setAttribute('aria-label',lang==='pt-BR'?'Navegação principal':'Main navigation');
  document.querySelector('.guide-contents nav').setAttribute('aria-label',lang==='pt-BR'?'Conteúdo do guia':'Guide contents');
  document.querySelector('.languages').setAttribute('aria-label',lang==='pt-BR'?'Idioma':'Language');
  document.querySelector('.guide-article img').alt=lang==='pt-BR'?'Biblioteca real do aplicativo com cartões de mods, busca, filtros e interruptores':'Actual app library with mod cards, search, filters, and on/off switches';
  params.set('lang',lang);
  history.replaceState({},'',`${location.pathname}?${params}${location.hash}`);
  scheduleCurrentSection();
}
const sectionLinks = [...document.querySelectorAll('.guide-contents a')];
const sections = sectionLinks.map(link => document.getElementById(link.hash.slice(1)));
let scrollFrame = 0;
function updateCurrentSection() {
  scrollFrame = 0;
  const headerBottom = document.querySelector('header').getBoundingClientRect().bottom;
  const readingLine = headerBottom + Math.min(96, innerHeight * 0.15);
  const atEnd = scrollY + innerHeight >= document.documentElement.scrollHeight - 2;
  const current = currentSectionIndex(sections.map(section => section.getBoundingClientRect().top), readingLine, atEnd);
  sectionLinks.forEach((link, index) => {
    if (index === current) {
      link.setAttribute('aria-current', 'location');
      link.setAttribute('aria-disabled', 'true');
    } else {
      link.removeAttribute('aria-current');
      link.removeAttribute('aria-disabled');
    }
  });
}
function scheduleCurrentSection() {
  if (!scrollFrame) scrollFrame = requestAnimationFrame(updateCurrentSection);
}
sectionLinks.forEach(link => link.addEventListener('click', event => {
  if (link.getAttribute('aria-disabled') === 'true') event.preventDefault();
}));
addEventListener('scroll', scheduleCurrentSection, {passive: true});
addEventListener('resize', scheduleCurrentSection);
new ResizeObserver(scheduleCurrentSection).observe(document.querySelector('.guide-article'));
new ResizeObserver(([entry]) => {
  document.documentElement.style.setProperty('--guide-header-height', `${entry.target.getBoundingClientRect().height}px`);
  scheduleCurrentSection();
}).observe(document.querySelector('header'));

document.querySelectorAll('[data-lang]').forEach(button=>button.addEventListener('click',()=>setLanguage(button.dataset.lang)));
setLanguage(params.get('lang')==='pt-BR'?'pt-BR':'en');
