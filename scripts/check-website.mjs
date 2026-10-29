import assert from 'node:assert/strict';
import { readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { languages, messages, themes } from '../website/i18n.mjs';

const root = fileURLToPath(new URL('../website/', import.meta.url));
const html = await readFile(path.join(root, 'index.html'), 'utf8');
const keys = Object.keys(messages['zh-CN']);
assert.deepEqual(languages, ['zh-CN', 'zh-TW', 'en']);
assert.equal(themes.length, 3);
for (const language of languages) {
  assert.deepEqual(Object.keys(messages[language]), keys, `${language} 字典键必须一致`);
  for (const [key, value] of Object.entries(messages[language])) assert.ok(typeof value === 'string' && value.trim(), `${language}.${key} 不得留空`);
}
for (const [, key] of html.matchAll(/data-i18n(?:-aria|-title|-alt)?="([^"]+)"/g)) assert.ok(keys.includes(key), `缺少文案：${key}`);
for (const [, url] of html.matchAll(/(?:src|href)="(\.\/[^"#]+)"/g)) await stat(path.resolve(root, url));
for (const [, anchor] of html.matchAll(/href="#([^"]+)"/g)) assert.ok(html.includes(`id="${anchor}"`), `缺少锚点：${anchor}`);
for (const [, id] of html.matchAll(/<use href="#([^"]+)"/g)) assert.ok(html.includes(`id="${id}"`), `缺少图标：${id}`);
for (const name of ['editor.webp', 'database.webp', 'mobile.webp']) assert.ok((await stat(path.join(root, 'assets', name))).size > 1000);
for (const name of ['three.module.min.js', 'three.core.min.js', 'LICENSE']) assert.ok((await stat(path.join(root, 'vendor/three', name))).size > 1000);
const license = await readFile(path.join(root, 'vendor/three/LICENSE'), 'utf8');
assert.match(license, /MIT License/);
assert.match(html, /<html lang="zh-CN"/);
assert.match(html, /0\.0\.6/);
assert.match(html, /0\.0\.7/);
const downloads = [...html.matchAll(/href="https:\/\/github.com\/a602017206\/qingjian\/releases\/latest\/download\/([^"]+)"/g)].map(match => match[1]);
assert.deepEqual(downloads.sort(), ['qingjian-macos-arm64.zip', 'qingjian-macos-x64.zip', 'qingjian-windows-x64.exe', 'qingjian-linux-x64.deb', 'qingjian-linux-x64.AppImage', 'qingjian-android-arm64.apk', 'qingjian-ios-unsigned-arm64.ipa'].sort());
assert.ok(!/<script[^>]+src="https?:/.test(html), '脚本应由本站提供');
assert.deepEqual([...html.matchAll(/data-feature-scene="([^"]+)"/g)].map(match => match[1]).sort(), ['blocks', 'database', 'history', 'links']);
for (const file of ['feature-scenes.mjs', 'rotation.mjs', 'scene-kit.mjs', 'cursor-trail.mjs']) await stat(path.join(root, file));
console.log(`官网静态检查通过：${languages.length} 种语言、${keys.length} 个文案键、${themes.length} 套主题、${downloads.length} 个真实平台下载入口，站内资源齐全。`);
