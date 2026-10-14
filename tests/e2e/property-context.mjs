import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { spawn } from 'node:child_process';
import { mkdir } from 'node:fs/promises';
import path from 'node:path';
const addr='127.0.0.1:3426',base=`http://${addr}`;
const server=spawn(path.resolve(`target/debug/server${process.platform==='win32'?'.exe':''}`),[],{env:{...process.env,QINGJIAN_DEV_ADDR:addr,QINGJIAN_DATA_DIR:path.resolve('.test-data',`property-context-${Date.now()}`)},stdio:'pipe'});
let browser,workspaceId;
const api=async(action,data={})=>{const response=await fetch(`${base}/api/${action}`,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({workspaceId,...data})});const result=await response.json();if(result.error)throw new Error(result.error);return result.data;};
try{
 await mkdir('test-results',{recursive:true});
 for(let i=0;i<100;i++){try{if((await fetch(base)).ok)break;}catch{}await new Promise(resolve=>setTimeout(resolve,100));}
 browser=await chromium.launch({headless:true,...(process.platform==='darwin'?{channel:'chrome'}:{})});
 const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];page.on('pageerror',error=>errors.push(error.message));
 await page.goto(base);await page.waitForSelector('.home-heading');
 await page.locator('.workspace-switch').click();await page.locator('[data-action="newWorkspace"]').click();await page.locator('#workspace-form [name="name"]').fill('子页属性验证');await page.locator('#workspace-form [type="submit"]').click();await page.waitForFunction(()=>document.querySelector('.workspace-switch strong')?.textContent==='子页属性验证');workspaceId=(await api('bootstrap')).activeId;
 await page.locator('.sidebar-bottom [data-action="import"]').click();const chooser=page.waitForEvent('filechooser');await page.locator('[data-action="chooseImport"]').click();await (await chooser).setFiles('tests/fixtures/notion-html.zip');await page.waitForSelector('.import-success');await page.locator('.modal-header [data-action="closeModal"]').click();
 await page.locator('.tree-entry .page-nav').click();const parentId=await page.locator('.tree-entry .page-nav').getAttribute('data-id');await page.locator('.db-name-cell [data-action="openRow"]').first().click();
 const panel=page.locator('.record-page-properties');await panel.waitFor();assert.match(await panel.innerText(),/评分/);assert.ok(await panel.locator('.option-blue').count());assert.ok(await panel.locator('.option-green').count());assert.equal(await panel.locator('.record-property-row').count(),4);
 assert.ok(!(await page.locator('.editor:not(.background-editor)').innerText()).includes('评分: 8'));assert.match(await page.locator('.editor:not(.background-editor)').innerText(),/正文一/);
 let pages=await api('pages'),parent=pages.find(p=>p.id===parentId),childId=parent.database.rows[0].pageId;const parentNav=()=>page.locator(`.tree-entry .page-nav[data-id="${parentId}"]`);const childNav=()=>page.locator(`.tree-entry .page-nav[data-id="${childId}"]`);


 const other=structuredClone(parent);other.id='';other.revision=0;other.title='另一父数据库';for(const row of other.database.rows)delete row.pageId;let otherParent=await api('save',{page:other});const otherChild=await api('save',{page:{title:'另一子页',parentId:otherParent.id,blocks:[]}});otherParent.database.rows[0].pageId=otherChild.id;otherParent=await api('save',{page:otherParent});
 await page.reload();await page.waitForSelector('.home-heading');await parentNav().click();await page.locator('.db-name-cell [data-action="openRow"]').first().click();await page.locator(`.tree-entry:has(.page-nav[data-id="${otherParent.id}"]) [data-action="expand"]`).click();
 let saving=false,release;const gate=new Promise(resolve=>release=resolve);await page.route('**/api/save',async route=>{if(route.request().postDataJSON().page.id===childId){saving=true;await gate;}await route.continue();});await page.locator('.editor [data-edit]').first().click();await page.locator('.block-input').fill('正文一，页面切换检查');await page.locator(`.page-nav[data-id="${otherChild.id}"]`).click();while(!saving)await new Promise(resolve=>setTimeout(resolve,20));
 try{assert.equal(await panel.getByRole('button',{name:'评分',exact:true}).isDisabled(),true,'导航等待期间不能打开旧父库的属性设置');assert.equal(await page.locator('#page-title').isEditable(),false);}finally{release();}
 await page.waitForFunction(()=>document.querySelector('#page-title')?.value==='另一子页'&&!document.querySelector('#page-title').readOnly);await page.unroute('**/api/save');assert.equal(await page.locator('#column-form').count(),0);const after=await api('pages');assert.equal(after.find(p=>p.id===otherParent.id).database.columns.find(c=>c.id==='col1').name,'评分');assert.deepEqual(errors,[]);console.log('延迟保存导航期间属性上下文和目标父库隔离验证通过');
}finally{await browser?.close();server.kill('SIGTERM');}
