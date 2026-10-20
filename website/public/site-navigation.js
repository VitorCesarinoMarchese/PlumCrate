export const navigationItems = [
  {key: 'features', href: './#features', en: 'The app', pt: 'O aplicativo'},
  {key: 'start', href: './#start', en: 'Get started', pt: 'Primeiros passos'},
  {key: 'guide', href: 'guide.html', en: 'User guide', pt: 'Guia de uso'},
  {key: 'github', href: 'https://github.com/VitorCesarinoMarchese/TS4ModManager', en: 'GitHub ↗', pt: 'GitHub ↗'},
];

export function renderSiteNavigation(language) {
  const nav = document.querySelector('header nav');
  nav.setAttribute('aria-label', language === 'pt-BR' ? 'Navegação principal' : 'Main navigation');
  nav.replaceChildren(...navigationItems.map(item => {
    const link = document.createElement('a');
    const url = new URL(item.href, location.href);
    if (url.origin === location.origin) url.searchParams.set('lang', language);
    link.href = url.href;
    link.textContent = language === 'pt-BR' ? item.pt : item.en;
    link.dataset.navKey = item.key;
    if (item.key === 'guide' && document.body.classList.contains('user-guide')) {
      link.setAttribute('aria-current', 'page');
    }
    return link;
  }));
}
