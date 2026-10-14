import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { spawn } from 'node:child_process';
import { mkdir } from 'node:fs/promises';
import path from 'node:path';
const addr='127.0.0.1:3422',base=`http://${addr}`;
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

 const source=process.env.QINGJIAN_TEST_HTML_ZIP||'tests/fixtures/notion-html.zip',records=process.env.QINGJIAN_TEST_HTML_ZIP?63:2;
 await page.locator('.sidebar-bottom [data-action="import"]').click();const chooser=page.waitForEvent('filechooser');await page.locator('[data-action="chooseImport"]').click();await (await chooser).setFiles(source);await page.waitForSelector('.import-success');assert.match(await page.locator('.import-success').innerText(),new RegExp(`${records+1} 篇笔记`));await page.locator('.modal-header [data-action="closeModal"]').click();
 assert.equal(await page.locator('.tree-entry .page-nav').count(),1);assert.equal(await page.locator('.page-row').count(),1);await page.locator('.tree-entry .page-nav').click();assert.equal(await page.locator('#page-title').inputValue(),'影视剧');assert.equal(await page.locator('.database-table tbody tr').count(),records);const parent=await page.locator('.tree-entry .page-nav').getAttribute('data-id');await page.locator(`.tree-entry:has(.page-nav[data-id="${parent}"]) [data-action="expand"]`).click();assert.equal(await page.locator('.tree-entry .page-nav').count(),records+1);if(!process.env.QINGJIAN_TEST_HTML_ZIP)await page.screenshot({path:'test-results/网页数据库与原始颜色.png',fullPage:true,animations:'disabled'});await page.locator(`.tree-entry:has(.page-nav[data-id="${parent}"]) [data-action="expand"]`).click();assert.equal(await page.locator('.tree-entry .page-nav').count(),1);
 if(!process.env.QINGJIAN_TEST_HTML_ZIP){
  const attachment=page.locator('[data-label="查看文档"] a[download]');
  await attachment.waitFor();
  assert.equal(await attachment.evaluate(async element=>await (await fetch(element.href)).text()),'匿名附件内容');
  const image=page.locator('.asset-preview img');await image.waitFor();
  await page.waitForFunction(()=>[...document.querySelectorAll('.asset-preview img')].every(el=>el.complete&&el.naturalWidth>0));
 }
 const status=page.locator('.database-table .option-input:has(select[aria-label="是否观看"])').first();assert.ok((await status.getAttribute('class')).includes('option-blue'));assert.equal(await page.locator('.database-table input[aria-label="类型"]').count(),0);assert.ok(await page.locator('.database-table [data-action="selectValues"]').count()>0);const dates=page.locator('.database-table input[aria-label="观看时间"]');assert.equal(await dates.first().getAttribute('type'),'date');const title=await page.locator('.database-table .db-name-cell input').first().inputValue();await page.locator('.db-name-cell [data-action="openRow"]').first().click();await page.waitForFunction(value=>document.querySelector('.modal')||document.querySelector('#page-title')?.value===value,title);assert.equal(await page.locator('#page-title').inputValue(),title,'数据库记录应打开对应导入子页面');assert.equal(await page.locator('#parent-select').inputValue(),parent);assert.equal(await page.locator('.record-page-properties').count(),1,'子页面应有结构化属性区');assert.ok((await page.locator('.record-page-properties').innerText()).includes('是否观看'));if(!process.env.QINGJIAN_TEST_HTML_ZIP)assert.match(await page.locator('.editor:not(.background-editor)').innerText(),/正文/);else assert.equal(await page.locator('.record-property-row').count(),10);assert.equal(await page.locator('.modal').count(),0);await page.reload();await page.waitForSelector('.home-heading');assert.equal(await page.locator('.tree-entry .page-nav').count(),1);assert.equal(await page.locator('.page-row').count(),1);assert.deepEqual(errors,[]);console.log(`网页影视导出界面验证通过：一个父页面、${records} 个子页面，折叠与记录打开正常`);
}finally{await browser?.close();server.kill('SIGTERM');}
