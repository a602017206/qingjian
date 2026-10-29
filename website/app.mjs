import { languages, themes, translate } from './i18n.mjs';

const root = document.documentElement;
const readPreference = key => { try { return localStorage.getItem(`qingjian-site-${key}`); } catch { return null; } };
const savePreference = (key, value) => { try { localStorage.setItem(`qingjian-site-${key}`, value); } catch { /* 私密浏览等环境仍可使用本次设置。 */ } };
let language = languages.includes(readPreference('language')) ? readPreference('language') : 'zh-CN';
let preview = 'editor';
let sceneController;
let featureController;
const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
let motionPaused = readPreference('motion') === 'paused' || reducedMotion.matches;
const languageSelect = document.querySelector('#language');
const menuToggle = document.querySelector('#menu-toggle');
const mobileNav = document.querySelector('#mobile-nav');
const motionButton = document.querySelector('#motion-toggle');
const featureMotionButton = document.querySelector('#feature-motion-toggle');
const featureGrid = document.querySelector('.feature-grid');
const resetButton = document.querySelector('#scene-reset');
const screenshot = document.querySelector('#product-screenshot');
const dialogScreenshot = document.querySelector('#dialog-screenshot');
const dialog = document.querySelector('#screenshot-dialog');
const t = key => translate(language, key);

function applyTranslations() {
  root.lang = language;
  languageSelect.value = language;
  for (const element of document.querySelectorAll('[data-i18n]')) element.textContent = t(element.dataset.i18n);
  for (const [data, attribute] of [['i18nAria', 'aria-label'], ['i18nTitle', 'title'], ['i18nAlt', 'alt']]) {
    for (const element of document.querySelectorAll(`[data-${data.replace(/[A-Z]/g, letter => `-${letter.toLowerCase()}`)}]`)) element.setAttribute(attribute, t(element.dataset[data]));
  }
  document.title = t('meta.title');
  document.querySelector('meta[name="description"]').content = t('meta.description');
  document.querySelector('meta[property="og:title"]').content = t('meta.title');
  document.querySelector('meta[property="og:description"]').content = t('meta.description');
  document.querySelector('meta[property="og:locale"]').content = { 'zh-CN': 'zh_CN', 'zh-TW': 'zh_TW', en: 'en_US' }[language];
  menuToggle.setAttribute('aria-label', t(mobileNav.hidden ? 'nav.menu' : 'nav.close'));
  updateMotionLabel();
  featureController?.setLanguage?.();
}

function setTheme(theme) {
  if (!themes.includes(theme)) return;
  root.dataset.theme = theme;
  for (const button of document.querySelectorAll('[data-set-theme]')) button.setAttribute('aria-pressed', String(button.dataset.setTheme === theme));
  document.querySelector('meta[name="theme-color"]').content = { paper: '#f7f8fc', midnight: '#131b2c', grove: '#f0f6f2' }[theme];
  sceneController?.setTheme?.(theme);
  featureController?.setTheme?.(theme);
}

function setPreview(value, focus = false) {
  if (!['editor', 'database', 'mobile'].includes(value)) return;
  preview = value;
  const panel = document.querySelector('#preview-panel');
  panel.classList.toggle('is-mobile', preview === 'mobile');
  panel.setAttribute('aria-labelledby', `tab-${preview}`);
  for (const button of document.querySelectorAll('[data-preview]')) {
    const selected = button.dataset.preview === preview;
    button.setAttribute('aria-selected', String(selected));
    button.tabIndex = selected ? 0 : -1;
    if (selected && focus) button.focus();
  }
  const size = { editor: [1600, 1100], database: [1440, 1050], mobile: [390, 844] }[preview];
  for (const img of [screenshot, dialogScreenshot]) {
    img.src = new URL(`./assets/${preview}.webp`, import.meta.url).href;
    img.width = size[0]; img.height = size[1];
    img.dataset.i18nAlt = `preview.alt.${preview}`;
    img.alt = t(`preview.alt.${preview}`);
  }
  const caption = document.querySelector('#preview-caption');
  caption.dataset.i18n = `preview.caption.${preview}`;
  caption.textContent = t(caption.dataset.i18n);
}

function closeMenu({ focus = false } = {}) {
  mobileNav.hidden = true;
  menuToggle.setAttribute('aria-expanded', 'false');
  menuToggle.setAttribute('aria-label', t('nav.menu'));
  if (focus) menuToggle.focus();
}

function updateMotionLabel() {
  for (const button of document.querySelectorAll('[data-motion-control]')) {
    const label = button.querySelector('[data-motion-label]');
    const controller = button.dataset.motionControl === 'features' ? featureController : sceneController;
    label.dataset.i18n = controller?.available === false ? 'motion.unavailable' : motionPaused ? 'motion.play' : 'motion.pause';
    label.textContent = t(label.dataset.i18n);
    button.setAttribute('aria-pressed', String(motionPaused));
    button.querySelector('.motion-symbol').textContent = motionPaused ? '▷' : 'Ⅱ';
  }
}

function pauseMotion(value) {
  motionPaused = value;
  sceneController?.setPaused?.(value); featureController?.setPaused?.(value); updateMotionLabel();
}

for (const button of document.querySelectorAll('[data-set-theme]')) button.addEventListener('click', () => {
  setTheme(button.dataset.setTheme); savePreference('theme', button.dataset.setTheme);
});
languageSelect.addEventListener('change', () => {
  language = languages.includes(languageSelect.value) ? languageSelect.value : 'zh-CN';
  savePreference('language', language); applyTranslations();
});
for (const button of document.querySelectorAll('[data-preview]')) {
  button.addEventListener('click', () => setPreview(button.dataset.preview));
  button.addEventListener('keydown', event => {
    const values = ['editor', 'database', 'mobile'];
    let index = values.indexOf(preview);
    if (event.key === 'ArrowRight') index = (index + 1) % values.length;
    else if (event.key === 'ArrowLeft') index = (index + values.length - 1) % values.length;
    else if (event.key === 'Home') index = 0;
    else if (event.key === 'End') index = values.length - 1;
    else return;
    event.preventDefault(); setPreview(values[index], true);
  });
}
menuToggle.addEventListener('click', () => {
  mobileNav.hidden = !mobileNav.hidden;
  menuToggle.setAttribute('aria-expanded', String(!mobileNav.hidden));
  menuToggle.setAttribute('aria-label', t(mobileNav.hidden ? 'nav.menu' : 'nav.close'));
});
for (const anchor of mobileNav.querySelectorAll('a')) anchor.addEventListener('click', () => closeMenu());
document.addEventListener('keydown', event => { if (event.key === 'Escape' && !mobileNav.hidden) closeMenu({ focus: true }); });
matchMedia('(min-width: 801px)').addEventListener('change', event => { if (event.matches) closeMenu(); });
document.querySelector('#screenshot-open').addEventListener('click', () => { dialog.showModal(); document.body.classList.add('dialog-open'); });
document.querySelector('#screenshot-close').addEventListener('click', () => dialog.close());
dialog.addEventListener('close', () => document.body.classList.remove('dialog-open'));
dialog.addEventListener('click', event => { if (event.target === dialog) dialog.close(); });
for (const button of [motionButton, featureMotionButton]) button.addEventListener('click', () => {
  pauseMotion(!motionPaused); savePreference('motion', motionPaused ? 'paused' : 'playing');
});
resetButton.addEventListener('click', () => sceneController?.resetRotation?.());
reducedMotion.addEventListener('change', event => {
  if (event.matches) {
    pauseMotion(true);
    root.classList.remove('motion-ready');
  }
});

applyTranslations();
setTheme(themes.includes(root.dataset.theme) ? root.dataset.theme : 'paper');

// 正文先可用，较重的 WebGL 模块异步加载，失败保留 CSS 纸页。
async function loadScene() {
  try {
    const { createScene } = await import('./scene.mjs');
    sceneController = createScene(document.querySelector('#scene'), { theme: root.dataset.theme, paused: motionPaused });
    if (sceneController.available) {
      motionButton.hidden = false; resetButton.hidden = false;
      document.querySelector('#scene-hint').hidden = false;
      document.querySelector('#scene').tabIndex = 0;
    }
  } catch {
    document.querySelector('#scene').dataset.ready = 'fallback';
    sceneController = { available: false };
  }
  updateMotionLabel();
}
loadScene();
document.querySelector('#scene').addEventListener('scene-unavailable', () => {
  motionButton.hidden = true; resetButton.hidden = true;
  document.querySelector('#scene-hint').hidden = true; document.querySelector('#scene').tabIndex = -1;
  updateMotionLabel();
});

async function loadFeatureScenes() {
  try {
    const { createFeatureScenes } = await import('./feature-scenes.mjs');
    featureController = createFeatureScenes(featureGrid, { theme: root.dataset.theme, paused: motionPaused, text: t });
    featureMotionButton.hidden = !featureController.available;
  } catch { featureGrid.dataset.effectsReady = 'fallback'; featureController = { available: false }; }
  updateMotionLabel();
}
featureGrid.addEventListener('scene-unavailable', () => { featureMotionButton.hidden = true; updateMotionLabel(); });
if ('IntersectionObserver' in window) {
  const lazyFeatures = new IntersectionObserver(entries => {
    if (entries.some(entry => entry.isIntersecting)) { lazyFeatures.disconnect(); loadFeatureScenes(); }
  }, { rootMargin: '300px' });
  lazyFeatures.observe(featureGrid);
} else loadFeatureScenes();

if ('IntersectionObserver' in window) {
  if (!reducedMotion.matches) root.classList.add('motion-ready');
  const observer = new IntersectionObserver(entries => {
    for (const entry of entries) if (entry.isIntersecting) { entry.target.classList.add('is-visible'); observer.unobserve(entry.target); }
  }, { threshold: .07 });
  for (const element of document.querySelectorAll('.reveal')) observer.observe(element);
}
