import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createHash } from 'node:crypto';
const imageHash = data => createHash('sha256').update(data).digest('hex');

const root = fileURLToPath(new URL('../../website/', import.meta.url));
const output = fileURLToPath(new URL('../../test-results/website/', import.meta.url));
const mime = { '.html': 'text/html; charset=utf-8', '.css': 'text/css', '.mjs': 'text/javascript', '.js': 'text/javascript', '.svg': 'image/svg+xml', '.webp': 'image/webp', '.txt': 'text/plain', '.xml': 'application/xml' };
const server = createServer(async (request, response) => {
  const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
  if (!pathname.startsWith('/qingjian/')) { response.writeHead(404); response.end(); return; }
  const file = path.resolve(root, pathname.slice('/qingjian/'.length) || 'index.html');
  if (!file.startsWith(root)) { response.writeHead(403); response.end(); return; }
  try { const content = await readFile(file); response.writeHead(200, { 'Content-Type': mime[path.extname(file)] || 'application/octet-stream' }); response.end(content); }
  catch { response.writeHead(404); response.end(); }
});
await mkdir(output, { recursive: true });
let browser;
const checks = [];
try {
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const localBase = `http://127.0.0.1:${server.address().port}/qingjian/`;
  const base = process.env.QINGJIAN_SITE_URL || localBase;
  const origin = new URL(base).origin;
  // 公网验收沿用命令行网络代理；Chrome 不会自动读取 HTTPS_PROXY。
  const proxyAddress = process.env.QINGJIAN_SITE_URL && (process.env.HTTPS_PROXY || process.env.https_proxy || process.env.HTTP_PROXY || process.env.http_proxy);
  const proxyUrl = proxyAddress ? new URL(proxyAddress) : null;
  const proxy = proxyUrl ? { server: `${proxyUrl.protocol}//${proxyUrl.host}`, ...(proxyUrl.username ? { username: decodeURIComponent(proxyUrl.username), password: decodeURIComponent(proxyUrl.password) } : {}) } : undefined;
  browser = await chromium.launch({ headless: true, ...(process.platform === 'darwin' ? { channel: 'chrome' } : {}), ...(proxy ? { proxy } : {}), args: ['--enable-webgl', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, reducedMotion: 'no-preference' });
  const errors = [], failed = [], external = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('response', response => { if (response.status() >= 400) failed.push(`${response.status()} ${response.url()}`); });
  page.on('request', request => { if (new URL(request.url()).origin !== origin) external.push(request.url()); });
  await page.goto(base, { waitUntil: 'networkidle' });
  assert.equal(await page.locator('html').getAttribute('lang'), 'zh-CN');
  await page.waitForSelector('#scene[data-ready="true"]');
  const canvas = page.locator('#scene canvas');
  // 先确认动态状态，再暂停截图；避免软件 WebGL 连续绘制干扰截图稳定等待。
  await page.waitForFunction(() => document.querySelector('#scene').dataset.animating === 'true');
  await page.locator('#motion-toggle').click();
  await page.waitForFunction(() => getComputedStyle(document.querySelector('#scene canvas')).opacity === '1');
  assert.ok((await canvas.screenshot()).length > 10000, 'WebGL 画布应真实绘制纸页而非空白');
  checks.push('项目子路径资源与真实 WebGL 渲染');

  assert.equal(await page.locator('[data-feature-scene]').count(), 4, '四张功能卡片应各有 Three.js 场景');
  const paperBefore = await canvas.screenshot();
  const paperBox = await canvas.boundingBox();
  await page.mouse.move(paperBox.x + paperBox.width * .48, paperBox.y + paperBox.height * .48);
  await page.mouse.down();
  await page.mouse.move(paperBox.x + paperBox.width * .72, paperBox.y + paperBox.height * .58, { steps: 8 });
  await page.mouse.up();
  const paperDragged = await canvas.screenshot();
  assert.notEqual(imageHash(paperDragged), imageHash(paperBefore), '暂停时仍应允许用户主动拖动纸页');
  await page.locator('#scene-reset').click();
  const paperReset = await canvas.screenshot();
  assert.notEqual(imageHash(paperReset), imageHash(paperDragged), '复位应将已旋转的纸页重新绘制');
  await page.locator('#scene').focus();
  await page.keyboard.press('ArrowLeft');
  await page.locator('#scene').evaluate(element => element.blur());
  const paperKeyed = await canvas.screenshot();
  assert.notEqual(imageHash(paperKeyed), imageHash(paperReset), '方向键应能旋转纸页');
  await page.locator('#scene').focus();
  await page.keyboard.press('Home');
  await page.locator('#scene').evaluate(element => element.blur());
  assert.notEqual(imageHash(await canvas.screenshot()), imageHash(paperKeyed), 'Home 应复位实际纸页模型，精确角度归零另由模型测试验证');
  checks.push('鼠标拖动纸页、暂停下主动旋转、方向键与复位');

  await page.locator('[data-feature-scene="blocks"]').scrollIntoViewIfNeeded();
  await page.waitForSelector('.feature-grid[data-effects-ready="true"]');
  assert.equal(await page.locator('.feature-effects-canvas').count(), 1, '四张卡片共用一个 WebGL 画布');
  await page.locator('#feature-motion-toggle').click();
  await page.waitForFunction(() => document.querySelector('.feature-grid').dataset.animating === 'true');
  await page.locator('#feature-motion-toggle').click();
  assert.equal(await page.locator('.feature-grid').getAttribute('data-animating'), 'false');
  for (const kind of ['blocks', 'database', 'links', 'history']) {
    const target = page.locator(`[data-feature-scene="${kind}"]`);
    await target.scrollIntoViewIfNeeded();
    assert.ok((await target.screenshot()).length > 2000, `${kind} 场景应真实绘制`);
  }
  await page.locator('#features').screenshot({ path: path.join(output, 'feature-effects.png') });
  await page.evaluate(() => window.scrollTo({ top: 0, behavior: 'instant' }));
  checks.push('四种功能场景共享画布、实际渲染与统一动效暂停');

  for (const theme of ['paper', 'midnight', 'grove']) {
    await page.locator(`[data-set-theme="${theme}"]`).click();
    assert.equal(await page.locator('html').getAttribute('data-theme'), theme);
    assert.equal(await page.locator(`[data-set-theme="${theme}"]`).getAttribute('aria-pressed'), 'true');
    for (const language of ['zh-CN', 'zh-TW', 'en']) {
      await page.locator('#language').selectOption(language);
      assert.equal(await page.locator('html').getAttribute('lang'), language);
      assert.match(await page.title(), language === 'en' ? /Qingjian/ : language === 'zh-TW' ? /輕箋/ : /轻笺/);
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `${theme}/${language} 桌面不应横向溢出`);
      assert.ok((await page.locator('h1').innerText()).length > 10);
    }
    await page.locator('#language').selectOption('zh-CN');
    await page.screenshot({ path: path.join(output, `desktop-${theme}.png`) });
  }
  const groveCanvas = await canvas.screenshot();
  await page.locator('[data-set-theme="midnight"]').click();
  assert.notEqual(imageHash(await canvas.screenshot()), imageHash(groveCanvas), '主题应改变 3D 场景的实际渲染');
  await page.locator('#motion-toggle').click();
  await page.locator('#language').selectOption('en');
  await page.reload({ waitUntil: 'networkidle' });
  assert.equal(await page.locator('html').getAttribute('lang'), 'en');
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'midnight');
  checks.push('九种主题语言组合与刷新持久化，主题驱动 3D 材质');

  await page.locator('#tab-database').click();
  assert.match(await page.locator('#product-screenshot').getAttribute('src'), /database.webp$/);
  await page.locator('#tab-database').press('ArrowRight');
  assert.equal(await page.locator('#tab-mobile').getAttribute('aria-selected'), 'true');
  assert.match(await page.locator('#product-screenshot').getAttribute('src'), /mobile.webp$/);
  await page.locator('#screenshot-open').click();
  assert.equal(await page.locator('#screenshot-dialog').isVisible(), true);
  await page.keyboard.press('Escape');
  assert.equal(await page.locator('#screenshot-dialog').isVisible(), false);
  await page.locator('#tab-mobile').press('Home');
  assert.equal(await page.locator('#tab-editor').getAttribute('aria-selected'), 'true');
  checks.push('截图分类、方向键、Home 与完整截图对话框');

  await page.evaluate(() => window.scrollTo({ top: 0, behavior: 'instant' }));
  await page.locator('#motion-toggle').click();
  assert.equal(await page.locator('#motion-toggle').getAttribute('aria-pressed'), 'true');
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('#scene[data-ready="true"]');
  assert.equal(await page.locator('#scene').getAttribute('data-animating'), 'false');
  await page.locator('#motion-toggle').click();
  await page.waitForFunction(() => document.querySelector('#scene').dataset.animating === 'true');
  await page.locator('#download').scrollIntoViewIfNeeded();
  await page.waitForFunction(() => document.querySelector('#scene').dataset.animating === 'false');
  checks.push('暂停偏好持久化与离屏停止渲染');

  await page.locator('#language').selectOption('zh-CN');
  await page.locator('[data-set-theme="paper"]').click();
  for (const width of [320, 375, 390, 768, 1024]) {
    await page.setViewportSize({ width, height: 844 });
    for (const language of ['zh-CN', 'zh-TW', 'en']) {
      await page.locator('#language').selectOption(language);
      const overflow = await page.evaluate(() => [...document.querySelectorAll('body *')].filter(element => {
        const rect = element.getBoundingClientRect(); return rect.width && (rect.right > innerWidth + 1 || rect.left < -1) && element.tagName !== 'symbol';
      }).map(element => ({ tag: element.tagName, class: element.className?.baseVal ?? element.className, text: element.textContent.slice(0, 90), width: Math.round(element.getBoundingClientRect().width), right: Math.round(element.getBoundingClientRect().right) })).slice(0, 15));
      if (!(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth))) await page.screenshot({ path: path.join(output, 'overflow.png'), fullPage: true });
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `${width}px/${language} 不应横向溢出：${JSON.stringify(overflow)}`);
    }
    if (width <= 768) {
      await page.locator('#menu-toggle').click();
      assert.equal(await page.locator('#mobile-nav').isVisible(), true);
      await page.keyboard.press('Escape');
      assert.equal(await page.locator('#mobile-nav').isVisible(), false);
      await page.locator('#menu-toggle').click();
      await page.locator('#mobile-nav a[href="#download"]').click();
      assert.equal(await page.locator('#mobile-nav').isVisible(), false);
    }
  }
  await page.setViewportSize({ width: 390, height: 844 });
  await page.locator('#language').selectOption('zh-CN');
  await page.locator('[data-feature-scene="blocks"]').scrollIntoViewIfNeeded();
  if (await page.locator('#feature-motion-toggle').getAttribute('aria-pressed') === 'false') await page.locator('#feature-motion-toggle').click();
  await page.locator('[data-feature-scene="blocks"]').scrollIntoViewIfNeeded();
  await page.screenshot({ path: path.join(output, 'mobile-feature-effects.png') });
  await page.evaluate(() => { document.querySelectorAll('.reveal').forEach(element => element.classList.add('is-visible')); window.scrollTo({ top: 0, behavior: 'instant' }); });
  await page.screenshot({ path: path.join(output, 'mobile-paper.png'), fullPage: true });
  await page.locator('[data-set-theme="midnight"]').click();
  await page.locator('#language').selectOption('en');
  await page.screenshot({ path: path.join(output, 'mobile-midnight-en.png'), fullPage: true });
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.locator('[data-set-theme="paper"]').click();
  await page.locator('#language').selectOption('zh-CN');
  await page.evaluate(() => window.scrollTo({ top: 0, behavior: 'instant' }));
  await page.screenshot({ path: path.join(output, 'desktop-paper-full.png'), fullPage: true });
  checks.push('五种屏宽 × 三语言、移动导航与无横向溢出');

  assert.deepEqual(errors, [], '主页面不应有 JavaScript 错误');
  assert.deepEqual(failed, [], '官网资源不应有 HTTP 错误');
  assert.deepEqual(external, [], '运行时不应依赖第三方请求');
  // 每组独立释放图形上下文，避免软件渲染器同时维护多个页面。
  await page.close();

  const reduced = await browser.newPage({ reducedMotion: 'reduce', viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });
  await reduced.goto(base, { waitUntil: 'networkidle' });
  await reduced.waitForSelector('#scene[data-ready="true"]');
  assert.equal(await reduced.locator('#scene').getAttribute('data-animating'), 'false');
  assert.equal(await reduced.locator('#motion-toggle').getAttribute('aria-pressed'), 'true');
  assert.ok(await reduced.locator('.feature-card').first().evaluate(element => getComputedStyle(element).opacity === '1'));
  const touchCanvas = reduced.locator('#scene canvas'), touchBefore = await touchCanvas.screenshot(), touchBox = await touchCanvas.boundingBox();
  const input = await reduced.context().newCDPSession(reduced);
  const x = touchBox.x + touchBox.width * .4, y = touchBox.y + touchBox.height * .5;
  await input.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x, y, id: 1 }] });
  for (let step = 1; step <= 6; step++) await input.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ x: x + step * 12, y: y + step, id: 1 }] });
  await input.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  assert.notEqual(imageHash(await touchCanvas.screenshot()), imageHash(touchBefore), '手机横向触控应旋转纸页');
  await reduced.locator('#scene-reset').click();
  const scrollBefore = await reduced.evaluate(() => scrollY);
  await input.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x, y, id: 2 }] });
  for (let step = 1; step <= 6; step++) {
    await input.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ x: x + 1, y: y - step * 25, id: 2 }] });
    await reduced.waitForTimeout(20);
  }
  await input.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  await reduced.waitForFunction(before => scrollY > before + 30, scrollBefore);
  await reduced.locator('[data-feature-scene="blocks"]').scrollIntoViewIfNeeded();
  await reduced.waitForSelector('.feature-grid[data-effects-ready="true"]');
  assert.equal(await reduced.locator('.feature-grid').getAttribute('data-animating'), 'false');
  checks.push('手机横向旋转、竖向滚动、减少动态默认暂停与正文可见');
  await reduced.close();

  const fallback = await browser.newPage();
  await fallback.addInitScript(() => {
    const original = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function(type, ...args) { return /webgl/i.test(type) ? null : original.call(this, type, ...args); };
    Storage.prototype.getItem = () => { throw new Error('存储被浏览器禁用'); };
    Storage.prototype.setItem = () => { throw new Error('存储被浏览器禁用'); };
  });
  const fallbackErrors = []; fallback.on('pageerror', error => fallbackErrors.push(error.message));
  await fallback.goto(base, { waitUntil: 'networkidle' });
  await fallback.waitForSelector('#scene[data-ready="fallback"]');
  assert.equal(await fallback.locator('.paper-fallback').isVisible(), true);
  assert.equal(await fallback.locator('#download a[href*="qingjian-windows-x64.exe"]').count(), 1);
  await fallback.locator('#language').selectOption('en');
  assert.equal(await fallback.locator('html').getAttribute('lang'), 'en');
  await fallback.locator('[data-set-theme="grove"]').click();
  assert.equal(await fallback.locator('html').getAttribute('data-theme'), 'grove');
  await fallback.locator('[data-feature-scene="blocks"]').scrollIntoViewIfNeeded();
  await fallback.waitForSelector('.feature-grid[data-effects-ready="fallback"]');
  assert.ok(await fallback.locator('.sketch-title').isVisible(), 'WebGL 失败保留卡片静态示意');
  assert.deepEqual(fallbackErrors, []);
  checks.push('WebGL 不可用及存储禁用时，正文、下载与设置仍可用');
  await fallback.close();

  const noScript = await browser.newPage({ javaScriptEnabled: false });
  await noScript.goto(base, { waitUntil: 'networkidle' });
  assert.equal(await noScript.locator('h1').isVisible(), true);
  assert.equal(await noScript.locator('.noscript-note').isVisible(), true);
  assert.equal(await noScript.locator('#download a[href*="/latest/download/"]').count(), 7);
  assert.equal(await noScript.locator('.feature-card').first().isVisible(), true);
  checks.push('关闭 JavaScript 仍可阅读和下载');

  await writeFile(path.join(output, 'result.json'), JSON.stringify({ result: '通过', url: base, checks, errors, failed, external }, null, 2));
  console.log(`官网浏览器验证通过：${checks.length} 组流程，页面错误、资源失败和第三方运行时请求均为零。`);
} finally { await browser?.close(); await new Promise(resolve => server.close(resolve)); }
