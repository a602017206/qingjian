import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { spawn } from 'node:child_process';
import { mkdir,writeFile } from 'node:fs/promises';
import path from 'node:path';
const addr='127.0.0.1:3421',base=`http://${addr}`;
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
 await page.locator('[data-action="togglePageSelection"]').first().click();const target=page.locator(`.page-row [data-page-select="${items[35].id}"]`);await target.scrollIntoViewIfNeeded();
 async function watchClick(locator){await locator.scrollIntoViewIfNeeded();await page.evaluate(()=>{const pane=document.querySelector('.main-scroll'),side=document.querySelector('.sidebar-scroll'),row=document.querySelector('.page-row input:focus')?.closest('.page-row');window.selectionProbe={pane,side,row,tops:[],running:true};const sample=()=>{const p=window.selectionProbe;p.tops.push(document.querySelector('.main-scroll').scrollTop);if(p.running)requestAnimationFrame(sample);};requestAnimationFrame(sample);});await locator.evaluate(el=>window.selectionProbe.target=el);const before=await page.locator('.main-scroll').evaluate(el=>el.scrollTop);await locator.click();await page.evaluate(()=>new Promise(resolve=>{let frames=0;const tick=()=>{if(++frames<12)requestAnimationFrame(tick);else{window.selectionProbe.running=false;resolve();}};requestAnimationFrame(tick);}));const result=await page.evaluate(()=>({sameTarget:window.selectionProbe.target.isConnected,samePane:window.selectionProbe.pane===document.querySelector('.main-scroll'),sameSide:window.selectionProbe.side===document.querySelector('.sidebar-scroll'),tops:window.selectionProbe.tops}));assert.equal(result.sameTarget,true,'勾选应保留当前控件');assert.equal(result.samePane,true,'勾选不应重建主滚动容器');assert.equal(result.sameSide,true,'勾选不应重建侧栏');assert.ok(result.tops.every(top=>Math.abs(top-before)<1),`勾选期间位置变化：${result.tops}`);}
 await watchClick(target);await page.locator(`.sidebar-scroll [data-page-select="${items[35].id}"]`).scrollIntoViewIfNeeded();await watchClick(page.locator(`.sidebar-scroll [data-page-select="${items[35].id}"]`));assert.equal(await target.isChecked(),false);await watchClick(target);await watchClick(page.locator(`.page-row [data-page-select="${items[36].id}"]`));assert.equal(await target.isChecked(),true);assert.equal(await page.locator(`.sidebar-scroll [data-page-select="${items[35].id}"]`).isChecked(),true);assert.match(await page.locator('.page-selection-toolbar').innerText(),/已选择 2/);await watchClick(target);assert.equal(await target.isChecked(),false);await page.locator(`.page-row [data-page-select="${items[36].id}"]`).focus();await page.keyboard.press('Space');assert.equal(await page.locator(`.page-row [data-page-select="${items[36].id}"]`).isChecked(),false);await page.keyboard.press('Space');assert.equal(await page.locator(`.page-row [data-page-select="${items[36].id}"]`).evaluate(el=>el===document.activeElement),true);await watchClick(page.locator('.sidebar-selection-bar [data-action="clearPageSelection"]'));assert.equal(await page.locator('[data-page-select]:checked').count(),0);await watchClick(page.locator('.sidebar-selection-bar [data-action="selectAllPages"]'));assert.equal(await page.locator('.page-row [data-page-select]:checked').count(),64);assert.equal(await page.locator('.sidebar-scroll [data-page-select]:checked').count(),64);await watchClick(page.locator('.sidebar-selection-bar [data-action="clearPageSelection"]'));assert.deepEqual(errors,[]);console.log('连续勾选、同步计数和逐帧位置验证通过');
}finally{await browser?.close();server.kill('SIGTERM');}
