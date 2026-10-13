import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { spawn } from 'node:child_process';
import { mkdir,writeFile } from 'node:fs/promises';
import path from 'node:path';
const addr='127.0.0.1:3420',base=`http://${addr}`;
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

 const source=process.env.QINGJIAN_TEST_NOTION_ZIP||'tests/fixtures/notion-hierarchy.zip',records=process.env.QINGJIAN_TEST_NOTION_ZIP?63:2;
 await page.locator('.sidebar-bottom [data-action="import"]').click();const chooser=page.waitForEvent('filechooser');await page.locator('[data-action="chooseImport"]').click();await (await chooser).setFiles(source);await page.waitForSelector('.import-success');assert.match(await page.locator('.import-success').innerText(),new RegExp(`${records+1} 篇笔记`));await page.locator('.modal-header [data-action="closeModal"]').click();
 assert.equal(await page.locator('.tree-entry .page-nav').count(),1);assert.equal(await page.locator('.page-row').count(),1);await page.locator('.tree-entry .page-nav').click();assert.equal(await page.locator('#page-title').inputValue(),'影视剧');assert.equal(await page.locator('.database-table tbody tr').count(),records);const parent=await page.locator('.tree-entry .page-nav').getAttribute('data-id');await page.locator(`.tree-entry:has(.page-nav[data-id="${parent}"]) [data-action="expand"]`).click();assert.equal(await page.locator('.tree-entry .page-nav').count(),records+1);if(!process.env.QINGJIAN_TEST_NOTION_ZIP)await page.screenshot({path:'test-results/真实结构父子目录.png',fullPage:true,animations:'disabled'});await page.locator(`.tree-entry:has(.page-nav[data-id="${parent}"]) [data-action="expand"]`).click();assert.equal(await page.locator('.tree-entry .page-nav').count(),1);
 const title=await page.locator('.database-table .db-name-cell input').first().inputValue();await page.locator('.db-name-cell [data-action="openRow"]').first().click();await page.waitForFunction(value=>document.querySelector('.modal')||document.querySelector('#page-title')?.value===value,title);assert.equal(await page.locator('#page-title').inputValue(),title,'数据库记录应打开对应导入子页面');assert.equal(await page.locator('#parent-select').inputValue(),parent);assert.ok((await page.locator('.editor').innerText()).trim().length>0);assert.equal(await page.locator('.modal').count(),0);await page.reload();await page.waitForSelector('.home-heading');assert.equal(await page.locator('.tree-entry .page-nav').count(),1);assert.equal(await page.locator('.page-row').count(),1);assert.deepEqual(errors,[]);console.log(`影视导出界面验证通过：一个父页面、${records} 个子页面，折叠与记录打开正常`);
}finally{await browser?.close();server.kill('SIGTERM');}
