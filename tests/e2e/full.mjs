import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { spawn } from 'node:child_process';
import { mkdir,writeFile } from 'node:fs/promises';
import path from 'node:path';
const addr='127.0.0.1:3416',base=`http://${addr}`;
await mkdir('test-results',{recursive:true});
const server=spawn(path.resolve(`target/debug/server${process.platform==='win32'?'.exe':''}`),[],{env:{...process.env,QINGJIAN_DATA_DIR:path.resolve('.test-data',`run-${Date.now()}`),QINGJIAN_DEV_ADDR:addr},stdio:'pipe'});
let output='';server.stdout.on('data',b=>output+=b);server.stderr.on('data',b=>output+=b);
let browser;
try {
 for(let i=0;i<100;i++){try{const r=await fetch(base);if(r.ok)break;}catch{}if(server.exitCode!==null)throw new Error(output);await new Promise(resolve=>setTimeout(resolve,100));}
 browser=await chromium.launch({headless:true,...(process.platform==='darwin'?{channel:'chrome'}:{})});
 const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];
 page.on('pageerror',error=>errors.push(error.message));
 await page.goto(base);await page.waitForSelector('.home-heading');
 await page.screenshot({path:'test-results/desktop-home.png',fullPage:true});
 // 实际表单提交，并创建隔离的工作区。
 await page.locator('.workspace-switch').click();await page.locator('[data-action="newWorkspace"]').click();
 await page.locator('#workspace-form [name="name"]').fill('端到端测试');await page.locator('#workspace-form [type="submit"]').click();
 await page.waitForFunction(()=>document.querySelector('.workspace-switch strong')?.textContent==='端到端测试');
 assert.equal(await page.locator('.page-row').count(),0);
 // 块编辑、自动保存和刷新持久化。
 await page.locator('.home-heading [data-action="newPage"]').click();await page.locator('#page-title').fill('中文持久化笔记');
 await page.locator('.block-render').first().click();await page.locator('.block-input').fill('这是跨平台保存的正文，关联 [[阅读清单]]。');
 await page.keyboard.press('ControlOrMeta+s');await page.waitForTimeout(250);await page.locator('.sidebar [data-action="home"]').click();
 await page.reload();await page.waitForSelector('.home-heading');await page.locator('#home-search').fill('跨平台保存');await page.waitForTimeout(200);assert.equal(await page.locator('.page-row').count(),1);
 await page.locator('.page-row [data-action="page"]').click();assert.equal(await page.locator('#page-title').inputValue(),'中文持久化笔记');assert.match(await page.locator('.editor').innerText(),/跨平台保存/);
 // 待办块、版本历史与回收站。
 await page.locator('[data-action="addBlock"]').click();await page.locator('[data-type="todo"]').click();await page.locator('.block-input').fill('验证待办');await page.locator('#page-title').click();await page.locator('.todo-check').check();
 await page.locator('[data-action="commit"]').click();await page.waitForFunction(()=>!document.querySelector('[data-action="commit"]')?.disabled);await page.locator('[data-action="history"]').click();await page.waitForSelector('.history-list');assert.equal(await page.locator('.history-list>div').count(),1);await page.locator('.modal-header [data-action="closeModal"]').click();
 let releaseSync;const syncGate=new Promise(resolve=>releaseSync=resolve);await page.route('**/api/sync',async route=>{await syncGate;await route.continue();});await page.locator('.topbar [data-action="sync"]').click();await page.waitForFunction(()=>document.querySelector('.topbar [data-action="sync"]')?.disabled);try{assert.equal(await page.locator('#page-title').isEditable(),false,'同步期间应保护编辑内容不被刷新覆盖');}finally{releaseSync();}await page.waitForFunction(()=>!document.querySelector('.topbar [data-action="sync"]')?.disabled);await page.unroute('**/api/sync');
 await page.screenshot({path:'test-results/desktop-editor.png',fullPage:true});
 await page.locator('.topbar [data-action="more"]').click();await page.locator('[data-action="delete"]').click();await page.locator('[data-action="confirmDelete"]').click();await page.waitForSelector('.home-heading');await page.locator('.sidebar-bottom>[data-action="trash"]').click();await page.locator('[data-action="restorePage"]').click();await page.waitForFunction(()=>document.querySelectorAll('.trash-list>div').length===0);
 // 数据库三种视图、记录和属性编辑。
 await page.locator('.main-nav [data-action="templates"]').click();await page.locator('[data-template="project"]').click();await page.waitForSelector('.database-table');
 await page.locator('.database-toolbar [data-action="newRow"]').click();await page.locator('[data-cell][data-column="name"]').fill('移动端任务');await page.locator('[data-cell][data-column="status"]').selectOption('进行中');
 await page.locator('[data-view="board"]').click();assert.match(await page.locator('.board-view').innerText(),/进行中/);assert.equal(await page.locator('.board-card').count(),1);
 await page.locator('[data-view="list"]').click();assert.equal(await page.locator('.database-list [data-column="name"]').inputValue(),'移动端任务');
 await page.locator('[data-view="table"]').click();await page.locator('[data-action="addColumn"]').click();await page.locator('#column-form [name="name"]').fill('估算');await page.locator('#column-form [name="type"]').selectOption('number');await page.locator('#column-form [type="submit"]').click();assert.equal(await page.locator('.database-table th').count(),5);
 await page.locator('[data-action="commit"]').click();await page.waitForFunction(()=>!document.querySelector('[data-action="commit"]')?.disabled);
 // Notion 导入使用真实选择文件流程。
 await page.locator('.sidebar-bottom>[data-action="import"]').click();const chooser=page.waitForEvent('filechooser');await page.locator('[data-action="chooseImport"]').click();await (await chooser).setFiles('tests/fixtures/notion.zip');await page.waitForSelector('.import-success');assert.match(await page.locator('.import-success').innerText(),/3 篇笔记/);await page.locator('.modal-header [data-action="closeModal"]').click();
 await page.locator('.sidebar-bottom>[data-action="import"]').click();await page.locator('#import-format').selectOption('csv');const rawChooser=page.waitForEvent('filechooser');await page.locator('[data-action="chooseImport"]').click();await (await rawChooser).setFiles({name:'123',mimeType:'text/csv',buffer:Buffer.from('名称,状态\n无扩展名表格,进行中')});await page.waitForSelector('.import-success');assert.match(await page.locator('.import-success').innerText(),/1 篇笔记/);await page.locator('.modal-header [data-action="closeModal"]').click();
 await page.locator('.page-row [data-action="page"]').filter({hasText:'旅行'}).click();await page.waitForSelector('.asset-preview img');assert.equal(await page.locator('.asset-preview img').count(),1);await page.waitForFunction(()=>document.querySelector('.asset-preview img')?.complete);await page.locator('.editor a[href^="page:"]').click();assert.equal(await page.locator('#page-title').inputValue(),'清单');
 // 主题、国际化和移动尺寸使用同一界面。
 await page.locator('.sidebar-bottom>[data-action="settings"]').click();await page.locator('[data-tab="appearance"]').click();await page.locator('[data-value="dark"]').click();await page.waitForFunction(()=>document.documentElement.dataset.theme==='dark');assert.equal(await page.locator('html').getAttribute('data-theme'),'dark');await page.locator('#pref-language').selectOption('zh-TW');await page.waitForFunction(()=>document.documentElement.lang==='zh-TW');assert.equal(await page.locator('html').getAttribute('lang'),'zh-TW');await page.locator('#pref-language').selectOption('zh-CN');await page.waitForFunction(()=>document.documentElement.lang==='zh-CN');await page.locator('[data-value="light"]').click();await page.waitForFunction(()=>document.documentElement.dataset.theme==='light');await page.locator('.modal-header [data-action="closeModal"]').click();
 await page.setViewportSize({width:390,height:844});assert.equal(await page.locator('.mobile-menu').isVisible(),true);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth),true);await page.locator('.mobile-menu').click();assert.equal(await page.locator('.sidebar.visible').count(),1);await page.locator('.sidebar .main-nav [data-action="home"]').click();assert.equal(await page.locator('.sidebar.visible').count(),0);
 await page.locator('.toast').waitFor({state:'hidden'});await page.screenshot({path:'test-results/mobile-home.png',fullPage:true});
 await page.locator('.page-row [data-action="page"]').filter({hasText:'中文持久化笔记'}).click();await page.screenshot({path:'test-results/mobile-editor.png',fullPage:true});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth),true);
 assert.deepEqual(errors,[]);await writeFile('test-results/验证结果.json',JSON.stringify({result:'通过',checks:['多工作区','自动保存与刷新','正文搜索','待办块','同步期间编辑保护','版本历史','回收站','数据库表格看板列表','属性编辑','Notion与无扩展名文件导入','主题与国际化','手机导航与布局'],pageErrors:errors},null,2));console.log('界面端到端验证通过：12 组流程，浏览器错误为零');
}finally{await browser?.close();server.kill('SIGTERM');}
