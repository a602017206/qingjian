import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { spawn,execFileSync } from 'node:child_process';
import { mkdir,readFile,readdir } from 'node:fs/promises';
import path from 'node:path';
const addr='127.0.0.1:3442',base=`http://${addr}`,root=path.resolve('.test-data',`sync-guide-${Date.now()}`),remote=path.join(root,'remote.git'),data=path.join(root,'data');
await mkdir(root,{recursive:true});execFileSync('git',['init','--bare','--initial-branch=main',remote],{stdio:'pipe'});
const server=spawn(path.resolve(`target/debug/server${process.platform==='win32'?'.exe':''}`),[],{env:{...process.env,PATH:'',QINGJIAN_DEV_ADDR:addr,QINGJIAN_DATA_DIR:data},stdio:'pipe'});
let browser;
const api=async(action,body={})=>{const result=await(await fetch(`${base}/api/${action}`,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)})).json();if(result.error)throw new Error(result.error);return result.data;};
try{
 for(let i=0;i<100;i++){try{if((await fetch(base)).ok)break;}catch{}await new Promise(r=>setTimeout(r,100));}
 const first=await api('createWorkspace',{name:'第一台设备',remote}),workspaceId=first.id;
 const note=await api('save',{workspaceId,page:{title:'没有外部命令也能同步',blocks:[{id:'a',type:'paragraph',text:'第一台设备写下的内容'}]}});
 await api('sync',{workspaceId});const second=await api('createWorkspace',{name:'第二台设备',remote,mode:'clone'});
 let pages=await api('pages',{workspaceId:second.id});assert.equal(pages[0].blocks[0].text,'第一台设备写下的内容');
 pages[0].blocks[0].text='第二台设备修改后返回';await api('save',{workspaceId:second.id,page:pages[0]});await api('sync',{workspaceId:second.id});await api('sync',{workspaceId});
 assert.equal((await api('pages',{workspaceId})).find(p=>p.id===note.id).blocks[0].text,'第二台设备修改后返回');
 console.log('服务器命令查找路径为空时，两个独立工作区上传、克隆、修改、下载往返通过');
 const local=await api('createWorkspace',{name:'同步引导验证'});
 browser=await chromium.launch({headless:true,...(process.platform==='darwin'?{channel:'chrome'}:{})});const page=await browser.newPage({viewport:{width:390,height:844},isMobile:true,hasTouch:true});page.setDefaultTimeout(8000);
 await page.goto(`${base}/?workspace=${local.id}`);await page.waitForSelector('.home-heading');await page.locator('.topbar [data-action="sync"]').tap();
 await page.waitForSelector('#sync-setup-form');assert.match(await page.locator('.sync-intro').innerText(),/无需安装/);
 await page.locator('#sync-setup-form [name="remote"]').fill('https://github.com/example/notes.git');await page.locator('#sync-setup-form [name="token"]').fill('anonymous-test-token');
 let syncCalls=0;
 await page.route('**/api/updateWorkspace',async route=>{const body=route.request().postDataJSON();body.workspace.remote=remote;await route.continue({postData:JSON.stringify(body)});});
 await page.route('**/api/sync',async route=>{syncCalls++;await route.fulfill({contentType:'application/json',body:JSON.stringify({error:'模拟网络不可用，笔记已保留'})});});
 await page.locator('#sync-setup-form [type="submit"]').tap();await page.waitForSelector('.sync-setup-error:not([hidden])');assert.match(await page.locator('.sync-setup-error').innerText(),/未完成/);assert.equal(await page.locator('#sync-setup-form [name="remote"]').inputValue(),'https://github.com/example/notes.git');assert.equal(syncCalls,1);
 assert.ok(!(await page.evaluate(()=>JSON.stringify(localStorage))).includes('anonymous-test-token'));
 const configFiles=await readdir(data);for(const name of configFiles.filter(n=>n.endsWith('.json')))assert.ok(!(await readFile(path.join(data,name),'utf8')).includes('anonymous-test-token'));
 await page.screenshot({path:'test-results/手机同步配置与失败保留.png',animations:'disabled'});
 await page.locator('#sync-setup-form [name="remote"]').fill('https://name:secret@github.com/example/notes.git');await page.locator('#sync-setup-form [type="submit"]').tap();assert.equal(syncCalls,1,'拒绝将认证信息放入仓库地址');
 await page.locator('#sync-setup-form [name="remote"]').fill('https://github.com/example/notes.git');await page.locator('#sync-setup-form [value="clone"]').check();await page.locator('#sync-setup-form [name="name"]').fill('新设备下载验证');
 await page.route('**/api/createWorkspace',async route=>{const body=route.request().postDataJSON();body.remote=remote;await route.continue({postData:JSON.stringify(body)});});
 await page.locator('#sync-setup-form [type="submit"]').tap();await page.waitForSelector('#sync-setup-form',{state:'detached'});assert.equal(await page.locator('.workspace-switch strong').innerText(),'新设备下载验证');assert.match(await page.locator('.main-scroll').innerText(),/没有外部命令也能同步/);assert.ok((await api('bootstrap')).workspaces.some(w=>w.id===local.id),'下载不替换旧工作区');
 console.log('手机同步引导、真实失败保留、拒绝地址内认证、令牌不落盘及新设备下载通过');
}finally{await browser?.close();server.kill('SIGTERM');}
