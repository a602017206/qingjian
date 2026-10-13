import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { spawn } from 'node:child_process';
import { mkdir,writeFile } from 'node:fs/promises';
import path from 'node:path';
const addr='127.0.0.1:3419',base=`http://${addr}`;
await mkdir('test-results',{recursive:true});
const server=spawn(path.resolve(`target/debug/server${process.platform==='win32'?'.exe':''}`),[],{env:{...process.env,QINGJIAN_DATA_DIR:path.resolve('.test-data',`run-${Date.now()}`),QINGJIAN_DEV_ADDR:addr},stdio:'pipe'});
let output='';server.stdout.on('data',b=>output+=b);server.stderr.on('data',b=>output+=b);
let browser;
try {
 for(let i=0;i<100;i++){try{const r=await fetch(base);if(r.ok)break;}catch{}if(server.exitCode!==null)throw new Error(output);await new Promise(resolve=>setTimeout(resolve,100));}
 browser=await chromium.launch({headless:true,...(process.platform==='darwin'?{channel:'chrome'}:{})});
 const page=await browser.newPage({viewport:{width:1440,height:900}}),errors=[];
 page.on('pageerror',error=>errors.push(error.message));
 await page.goto(base);await page.waitForSelector('.home-heading');
 await page.screenshot({path:'test-results/desktop-home.png',fullPage:true,animations:'disabled'});
 // 实际表单提交，并创建隔离的工作区。
 await page.locator('.workspace-switch').click();await page.locator('[data-action="newWorkspace"]').click();
 await page.locator('#workspace-form [name="name"]').fill('端到端测试');await page.locator('#workspace-form [type="submit"]').click();
 await page.waitForFunction(()=>document.querySelector('.workspace-switch strong')?.textContent==='端到端测试');
 assert.equal(await page.locator('.page-row').count(),0);

 let workspaceId;const call=async(action,args={})=>{if(action!=='bootstrap'&&!workspaceId)workspaceId=(await call('bootstrap')).activeId;const r=await fetch(`${base}/api/${action}`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({workspaceId,...args})});const v=await r.json();if(v.error)throw new Error(v.error);return v.data;};
 const items=[];for(let i=0;i<64;i++)items.push(await call('save',{page:{title:'导入笔记'+String(i).padStart(2,'0'),blocks:[{id:'b',type:'paragraph',text:'原始正文'}]}}));const other=await call('createWorkspace',{name:'同仓库副本'});for(const source of items.slice(0,2))await call('save',{workspaceId:other.id,page:source});await call('activate',{workspaceId});await page.reload();await page.waitForSelector('.home-heading');
 const target=page.locator(`.tree-entry .page-nav[data-id="${items[35].id}"]`);await target.scrollIntoViewIfNeeded();await target.click({button:'right'});const before=await page.locator('.sidebar-scroll').evaluate(el=>el.scrollTop);assert.ok(before>500);await page.locator('[data-action="pageMenuTrash"]').click();await page.locator('[data-action="pageMenuConfirmTrash"]').click();await page.waitForFunction(id=>!document.querySelector(`.page-nav[data-id="${id}"]`),items[35].id);assert.ok(await page.locator('.sidebar-scroll').evaluate(el=>el.scrollTop)>500,'删除后不应跳回顶部');
 await page.locator('[data-action="togglePageSelection"]').first().click();assert.equal(await page.locator('.page-row [data-action="page"]').first().evaluate(el=>getComputedStyle(el).textAlign),'left','批量模式中页面名称应保持左对齐');await page.locator(`.sidebar-scroll [data-page-select="${items[34].id}"]`).check();await page.locator(`.sidebar-scroll [data-page-select="${items[36].id}"]`).check();await page.locator('[data-action="trashSelectedPages"]').first().click();assert.match(await page.locator('.modal').innerText(),/2/);await page.locator('[data-action="confirmTrashSelected"]').click();await page.waitForFunction(()=>document.querySelectorAll('[data-page-select]:checked').length===0&&!document.querySelector('.modal'));
 const stored=await call('pages');assert.equal(stored.filter(p=>p.deleted).length,3);assert.equal(stored.filter(p=>!p.deleted).length,61);
 await page.screenshot({path:'test-results/批量管理与位置保持.png',fullPage:true,animations:'disabled'});await page.locator('.sidebar-bottom [data-action="trash"]').click();await page.waitForSelector('[data-action="emptyTrash"]');
 await page.locator(`[data-trash-select="${items[35].id}"]`).check();await page.locator('[data-action="purgeSelected"]').click();await page.waitForSelector('[data-action="confirmPurge"]');assert.match(await page.locator('.modal').innerText(),/无法.*回收站|不能.*回收站/);await page.locator('.modal-header [data-action="closeModal"]').click();assert.equal((await call('pages')).filter(p=>p.deleted).length,3);
 await page.locator('[data-action="emptyTrash"]').click();await page.waitForSelector('[data-action="confirmPurge"]');await page.screenshot({path:'test-results/清空回收站预览.png',fullPage:true,animations:'disabled'});await page.locator('[data-action="confirmPurge"]').click();await page.waitForFunction(()=>!document.querySelector('[data-action="emptyTrash"]'));assert.equal((await call('pages')).length,61);assert.match(await page.locator('.toast').innerText(),/释放/);
 await page.locator('.main-nav [data-action="home"]').click();await page.locator(`.sidebar-scroll [data-page-select="${items[0].id}"]`).check();await page.locator('.workspace-switch').click();await page.locator(`[data-action="activate"][data-id="${other.id}"]`).click();await page.waitForFunction(()=>document.querySelector('.workspace-switch strong')?.textContent==='同仓库副本');assert.equal(await page.locator('[data-page-select]:checked').count(),0,'切换工作区不能保留同标识页面的选择');assert.equal(await page.locator('.sidebar-selection-bar').count(),0);await page.locator('[data-action="togglePageSelection"]').first().click();await page.locator(`.sidebar-scroll [data-page-select="${items[1].id}"]`).check();await page.locator('[data-action="trashSelectedPages"]').first().click();await page.locator('[data-action="confirmTrashSelected"]').click();await page.waitForFunction(()=>document.querySelectorAll('[data-page-select]:checked').length===0&&!document.querySelector('.modal'));const otherPages=await call('pages',{workspaceId:other.id});assert.deepEqual(otherPages.filter(p=>p.deleted).map(p=>p.id),[items[1].id]);assert.equal((await call('pages')).find(p=>p.id===items[0].id).deleted,false);
 await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);assert.deepEqual(errors,[]);console.log('批量回收、永久清理与滚动位置界面验证通过');
}finally{await browser?.close();server.kill('SIGTERM');}
