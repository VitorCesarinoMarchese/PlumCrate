import {currentSectionIndex} from './guide-navigation.js';
import {initializeNavigation} from './site-navigation.js';

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

initializeNavigation();
scheduleCurrentSection();
