import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {spawn,execFileSync} from 'node:child_process';
import {mkdir,readFile} from 'node:fs/promises';
import path from 'node:path';
const addr='127.0.0.1:3443',base=`http://${addr}`,root=path.resolve('.test-data',`sync-conflicts-${Date.now()}`),data=path.join(root,'data'),remote=path.join(root,'remote.git');
await mkdir(root,{recursive:true});await mkdir('test-results',{recursive:true});execFileSync('git',['init','--bare','--initial-branch=main',remote],{stdio:'pipe'});
const server=spawn(path.resolve('target/debug/server'),[],{env:{...process.env,PATH:'',QINGJIAN_DEV_ADDR:addr,QINGJIAN_DATA_DIR:data},stdio:'pipe'});
const api=async(action,body={})=>{const result=await(await fetch(`${base}/api/${action}`,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)})).json();if(result.error)throw new Error(result.error);return result.data;};
let browser;
try{
 for(let i=0;i<100;i++){try{if((await fetch(base)).ok)break;}catch{}await new Promise(r=>setTimeout(r,100));}
 const a=await api('createWorkspace',{name:'电脑笔记',remote});let note=await api('save',{workspaceId:a.id,page:{id:'one',title:'影视剧',blocks:[{id:'p',type:'paragraph',text:'原始记录'}]}});await api('sync',{workspaceId:a.id});
 const b=await api('createWorkspace',{name:'手机笔记',remote,mode:'clone'});let local=(await api('pages',{workspaceId:b.id}))[0];local.blocks[0].text='手机修改的观看记录';await api('save',{workspaceId:b.id,page:local});note.blocks[0].text='电脑修改的观看记录';await api('save',{workspaceId:a.id,page:note});await api('sync',{workspaceId:a.id});
 browser=await chromium.launch({headless:true,...(process.platform==='darwin'?{channel:'chrome'}:{})});const page=await browser.newPage({viewport:{width:390,height:844},isMobile:true,hasTouch:true});page.setDefaultTimeout(10000);
 await page.goto(`${base}/?workspace=${b.id}`);await page.waitForSelector('.home-heading');await page.locator('.topbar [data-action="sync"]').tap();await page.waitForSelector('#sync-conflict-form');
 assert.match(await page.locator('.sync-version-grid').innerText(),/手机修改的观看记录/);assert.match(await page.locator('.sync-version-grid').innerText(),/电脑修改的观看记录/);
 const overflow=await page.locator('.sync-conflict').evaluate(el=>el.scrollWidth>el.clientWidth);assert.equal(overflow,false,'手机冲突卡片不横向溢出');
 await page.screenshot({path:'test-results/手机同步冲突版本核对.png',animations:'disabled'});
 await page.locator('[data-conflict-path="pages/one.json"]').selectOption('remote');await page.locator('#sync-conflict-form [type="submit"]').tap();await page.waitForSelector('#sync-conflict-form',{state:'detached'});
 assert.equal((await api('pages',{workspaceId:b.id}))[0].blocks[0].text,'电脑修改的观看记录');
 // 页面入口查看恢复点，并恢复到新工作区。
 await page.locator('.topbar [data-action="sidebar"]').tap();await page.locator('.workspace-switch').tap();await page.locator('[data-action="workspaceManage"]').tap();await page.locator(`[data-action="syncBackups"][data-id="${b.id}"]`).tap();
 await page.locator('[data-action="restoreSyncBackup"][data-side="local"]').first().tap();await page.waitForSelector('[data-action="restoreSyncBackup"]',{state:'detached'});assert.match(await page.locator('.main-scroll').innerText(),/影视剧/);
 const config=await api('bootstrap'),restored=config.workspaces.find(w=>w.id===config.activeId);assert.equal(restored.remote,'');assert.equal(new URL(page.url()).searchParams.get('workspace'),restored.id);assert.equal((await api('pages',{workspaceId:restored.id}))[0].blocks[0].text,'手机修改的观看记录');
 const manage=async()=>{await page.locator('.topbar [data-action="sidebar"]').tap();await page.locator('.workspace-switch').tap();await page.locator('[data-action="workspaceManage"]').tap();};
 await manage();await page.locator(`[data-action="removeWorkspace"][data-id="${restored.id}"]`).tap();await page.locator('#workspace-remove-form [type="submit"]').tap();await page.waitForSelector('#workspace-remove-form',{state:'detached'});
 assert.ok(!(await api('bootstrap')).workspaces.some(w=>w.id===restored.id));await manage();let archives=await api('archivedWorkspaces');let archive=archives.find(a=>a.workspace.id===restored.id);
 await page.locator(`[data-action="restoreWorkspace"][data-id="${archive.id}"]`).tap();await page.waitForSelector('.managed-workspaces',{state:'detached'});assert.equal((await api('pages',{workspaceId:restored.id}))[0].blocks[0].text,'手机修改的观看记录');
 await manage();await page.locator(`[data-action="removeWorkspace"][data-id="${restored.id}"]`).tap();await page.locator('#workspace-remove-form [type="submit"]').tap();await page.waitForSelector('#workspace-remove-form',{state:'detached'});await manage();archives=await api('archivedWorkspaces');archive=archives.find(a=>a.workspace.id===restored.id);
 await page.locator(`[data-action="purgeWorkspace"][data-id="${archive.id}"]`).tap();await page.locator('#workspace-purge-form [type="checkbox"]').check();await page.locator('#workspace-purge-form [type="submit"]').tap();await page.waitForSelector('#workspace-purge-form',{state:'detached'});assert.ok(!(await api('archivedWorkspaces')).some(a=>a.id===archive.id));
 await assert.rejects(readFile(path.join(data,'archived-workspaces',archive.id,'pages/one.json')));assert.ok((await api('pages',{workspaceId:b.id})).length,'永久删除恢复副本不影响原工作区');
 console.log('手机同页冲突选择、双方摘要、恢复点新工作区、本机移除恢复、永久清理通过；服务端没有外部命令查找路径');
 // 独立历史需明确选择；远程变化后旧选择不能覆盖新内容。
 const c=await api('createWorkspace',{name:'新装手机',remote});await api('save',{workspaceId:c.id,page:{id:'sample',title:'示例笔记',blocks:[]}});await page.goto(`${base}/?workspace=${c.id}`);await page.waitForSelector('.home-heading');await page.locator('.topbar [data-action="sync"]').tap();await page.waitForSelector('#sync-conflict-form');assert.equal(await page.locator('[name="strategy"]:checked').count(),0);
 await api('sync',{workspaceId:a.id});let current=(await api('pages',{workspaceId:a.id}))[0];current.blocks[0].text='远端刚刚又修改了';await api('save',{workspaceId:a.id,page:current});await api('sync',{workspaceId:a.id});
 await page.locator('[name="strategy"][value="remote"]').check();await page.locator('#sync-conflict-form [type="submit"]').tap();await page.waitForSelector('.sync-operation-error:not([hidden])');assert.match(await page.locator('.sync-operation-error').innerText(),/变化/);
 await page.locator('#sync-conflict-form [data-action="sync"]').tap();await page.waitForSelector('#sync-conflict-form [name="strategy"]:checked',{state:'detached'});await page.locator('[name="strategy"][value="remote"]').check();await page.locator('#sync-conflict-form [type="submit"]').tap();await page.waitForSelector('#sync-conflict-form',{state:'detached'});assert.equal((await api('pages',{workspaceId:c.id}))[0].blocks[0].text,'远端刚刚又修改了');
 console.log('新装手机独立历史选择、过时预览拒绝、重新检查后以远程为准通过');
}finally{await browser?.close();server.kill('SIGTERM');}
