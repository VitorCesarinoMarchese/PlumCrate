export const navigationItems = [
  {key: 'features', href: '/#features', en: 'The app', pt: 'O aplicativo'},
  {key: 'start', href: '/#start', en: 'Get started', pt: 'Primeiros passos'},
  {key: 'guide', href: '/guide', en: 'User guide', pt: 'Guia de uso'},
  {key: 'github', href: 'https://github.com/VitorCesarinoMarchese/TS4ModManager', en: 'GitHub ↗', pt: 'GitHub ↗'},
];

export function languageHref(language, pathname, search = '', hash = '') {
  const normalizedPath = pathname.replace(/\/$/, '').replace(/\.html$/, '');
  const guide = normalizedPath === '/guide' || normalizedPath === '/pt-br/guide';
  const path = language === 'pt-BR' ? `/pt-br${guide ? '/guide' : ''}` : guide ? '/guide' : '/';
  const params = new URLSearchParams(search);
  params.delete('lang');
  return `${path}${params.size ? `?${params}` : ''}${hash}`;
}

export function initializeNavigation() {
  const legacyLanguage = new URLSearchParams(location.search).get('lang');
  if (legacyLanguage === 'en' || legacyLanguage === 'pt-BR') {
    const destination = languageHref(legacyLanguage, location.pathname, location.search, location.hash);
    if (legacyLanguage !== document.documentElement.lang) location.replace(destination);
    else history.replaceState({}, '', destination);
  }
  for (const link of document.querySelectorAll('[data-lang]')) {
    link.href = languageHref(link.dataset.lang, location.pathname, location.search, location.hash);
  }
}

if (typeof window !== 'undefined') window.addEventListener('hashchange', initializeNavigation);
