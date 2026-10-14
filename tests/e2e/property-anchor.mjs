import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { spawn } from 'node:child_process';
import { mkdir,readFile } from 'node:fs/promises';
import path from 'node:path';
const addr='127.0.0.1:3429',base=`http://${addr}`;
const server=spawn(path.resolve(`target/debug/server${process.platform==='win32'?'.exe':''}`),[],{env:{...process.env,QINGJIAN_DEV_ADDR:addr,QINGJIAN_DATA_DIR:path.resolve('.test-data',`property-anchor-${Date.now()}`)},stdio:'pipe'});
let browser,workspaceId;
const api=async(action,data={})=>{const response=await fetch(`${base}/api/${action}`,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({workspaceId,...data})});const result=await response.json();if(result.error)throw new Error(result.error);return result.data;};
try{
 await mkdir('test-results',{recursive:true});
 for(let i=0;i<100;i++){try{if((await fetch(base)).ok)break;}catch{}await new Promise(resolve=>setTimeout(resolve,100));}
 browser=await chromium.launch({headless:true,...(process.platform==='darwin'?{channel:'chrome'}:{})});
 const page=await browser.newPage({viewport:{width:2536,height:1354}}),errors=[];page.on('pageerror',error=>errors.push(error.message));
 await page.goto(base);await page.waitForSelector('.home-heading');
 await page.locator('.workspace-switch').click();await page.locator('[data-action="newWorkspace"]').click();await page.locator('#workspace-form [name="name"]').fill('子页属性验证');await page.locator('#workspace-form [type="submit"]').click();await page.waitForFunction(()=>document.querySelector('.workspace-switch strong')?.textContent==='子页属性验证');workspaceId=(await api('bootstrap')).activeId;
 await page.locator('.sidebar-bottom [data-action="import"]').click();const chooser=page.waitForEvent('filechooser');await page.locator('[data-action="chooseImport"]').click();await (await chooser).setFiles('tests/fixtures/notion-html.zip');await page.waitForSelector('.import-success');await page.locator('.modal-header [data-action="closeModal"]').click();
 await page.locator('.tree-entry .page-nav').click();const parentId=await page.locator('.tree-entry .page-nav').getAttribute('data-id');await page.locator('[data-view="gallery"]').click();await page.locator('.gallery-card [data-action="openRow"]').first().click();
 const panel=page.locator('.record-page-properties');await panel.waitFor();assert.match(await panel.innerText(),/评分/);assert.ok(await panel.locator('.option-blue').count());assert.ok(await panel.locator('.option-green').count());
 assert.ok(!(await page.locator('.editor:not(.background-editor)').innerText()).includes('评分: 8'));assert.match(await page.locator('.editor:not(.background-editor)').innerText(),/正文一/);

 const assertAnchor=async(button,scope)=>{const anchor=await button.boundingBox();await button.evaluate(el=>{document.activeElement?.blur();el.click();});await page.waitForSelector('#column-form');const popup=await page.locator('.property-popover').boundingBox(),region=await scope.boundingBox();assert.ok(popup.x>=region.x-1,'新增属性必须在触发按钮所属区域内，不能定位到左上角');assert.ok(Math.abs(popup.x-anchor.x)<420,'弹层应靠近点击按钮');assert.ok(popup.x+popup.width<=page.viewportSize().width);return {anchor,popup};};
 await assertAnchor(panel.locator('[data-action="addColumn"]'),page.locator('.record-peek'));await page.locator('#column-form [name="name"]').fill('保持输入');await page.setViewportSize({width:1440,height:1000});await page.waitForTimeout(150);assert.equal(await page.locator('#column-form [name="name"]').inputValue(),'保持输入');const moved=await page.locator('.property-popover').boundingBox(),current=await panel.locator('[data-action="addColumn"]').boundingBox();assert.ok(Math.abs(moved.x-current.x)<80,'窗口变化后仍跟随真实按钮重新定位');await page.screenshot({path:'test-results/新增属性按钮相邻弹层.png',animations:'disabled'});await page.locator('.modal-header [data-action="closeModal"]').click();
 await page.locator('.record-peek [data-action="closeRecordPeek"]').click();await page.locator('[data-view="table"]').click();const tableAdd=page.locator('.database-table th [data-action="addColumn"]');await assertAnchor(tableAdd,page.locator('main'));await page.locator('.modal-header [data-action="closeModal"]').click();await page.locator('.db-name-cell [data-action="openRow"]').first().click();await page.setViewportSize({width:390,height:844});await panel.locator('[data-action="addColumn"]').evaluate(el=>{document.activeElement?.blur();el.click();});await page.waitForSelector('#column-form');const mobile=await page.locator('.property-popover').boundingBox();assert.ok(mobile.x>=0&&mobile.x+mobile.width<=390);await page.screenshot({path:'test-results/手机新增属性弹层.png',animations:'disabled'});assert.deepEqual(errors,[]);console.log('无焦点新增属性、窗口跟随、表头入口及手机边界定位验证通过');
}finally{await browser?.close();server.kill('SIGTERM');}
