import assert from 'node:assert/strict';import {chromium} from 'playwright';import {spawn,execFileSync} from 'node:child_process';import {mkdir} from 'node:fs/promises';import path from 'node:path';
const addr='127.0.0.1:3444',base=`http://${addr}`,root=path.resolve('.test-data','gitee-sync-'+Date.now()),remote=path.join(root,'remote.git');await mkdir(root,{recursive:true});await mkdir('test-results',{recursive:true});execFileSync('git',['init','--bare','--initial-branch=master',remote],{stdio:'pipe'});
const server=spawn(path.resolve('target/debug/server'),[],{env:{...process.env,PATH:'',QINGJIAN_DEV_ADDR:addr,QINGJIAN_DATA_DIR:path.join(root,'data')},stdio:'pipe'});let browser;
const api=async(action,request={})=>{const result=await(await fetch(`${base}/api/${action}`,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(request)})).json();if(result.error)throw new Error(result.error);return result.data;};
try{
 for(let i=0;i<100;i++){try{if((await fetch(base)).ok)break;}catch{}await new Promise(r=>setTimeout(r,100));}
 const source=await api('createWorkspace',{name:'模拟远端',branch:'master',remote});await api('save',{workspaceId:source.id,page:{title:'来自现有分支的笔记',blocks:[]}});await api('sync',{workspaceId:source.id});
 browser=await chromium.launch({headless:true,...(process.platform==='darwin'?{channel:'chrome'}:{})});
 for(const phone of [false,true]){
  const w=await api('createWorkspace',{name:phone?'手机测试':'桌面测试'}),page=await browser.newPage({viewport:phone?{width:390,height:844}:{width:1360,height:960},isMobile:phone,hasTouch:phone});const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.addInitScript(({remote,phone})=>{window.isTauri=true;window.nativeSyncCalls=0;window.__TAURI_INTERNALS__={transformCallback:()=>1,invoke:async(cmd,args)=>{
   if(cmd==='desktop_pending')return [];if(cmd==='desktop_locale')return null;if(cmd==='plugin:event|listen')return 1;if(cmd!=='dispatch')throw '测试环境不调用原生插件';if(args.action==='sync'){window.nativeSyncCalls++;throw '连接远程失败：Gitee 帐号或私人令牌不正确';}
   const request={...args.request};if(args.action==='createWorkspace'&&request.mode==='clone')request.remote=remote;
   const result=await(await fetch('/api/'+args.action,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(request)})).json();if(result.error)throw result.error;return args.action==='appInfo'?{...result.data,os:phone?'android':'macos'}:result.data;
  }};},{remote,phone});
  await page.route('https://api.github.com/**',route=>route.fulfill({status:503,body:'测试环境不访问更新服务'}));await page.goto(`${base}/?workspace=${w.id}`);await page.waitForSelector('.home-heading');await page.locator('.topbar [data-action="sync"]').click();await page.waitForSelector('#sync-setup-form');
  assert.equal(await page.locator('#sync-provider').inputValue(),'gitee');assert.equal(await page.locator('a[href="https://gitee.com/projects/new"]').count(),1);assert.equal(await page.locator('a[href="https://gitee.com/profile/personal_access_tokens"]').count(),1);
  await page.locator('[name="remote"]').fill('https://gitee.com/example/notes.git');await page.locator('[name="token"]').fill('anonymous-test-token');await page.locator('#sync-setup-form [type="submit"]').click();await page.waitForSelector('.sync-setup-error:not([hidden])');assert.match(await page.locator('.sync-setup-error').innerText(),/用户名/);assert.equal(await page.evaluate(()=>window.nativeSyncCalls),0);
  await page.locator('[name="username"]').fill('actual-user');await page.locator('#sync-setup-form [type="submit"]').click();await page.waitForFunction(()=>document.querySelector('.sync-setup-error')?.textContent.includes('帐号或私人令牌不正确'));
  const text=await page.locator('.sync-setup-error').innerText();assert.ok(!text.includes('undefined'));assert.ok(!text.includes('anonymous-test-token'));assert.equal(await page.locator('[name="remote"]').inputValue(),'https://gitee.com/example/notes.git');assert.ok(!(await page.evaluate(()=>JSON.stringify(localStorage))).includes('anonymous-test-token'));
  await page.locator('.sync-setup-error').scrollIntoViewIfNeeded();await page.screenshot({path:`test-results/${phone?'手机':'桌面'}原生同步错误与国内仓库.png`,animations:'disabled'});
  await page.locator('[name="mode"][value="clone"]').check();await page.locator('.sync-advanced summary').click();assert.equal(await page.locator('[name="branch"]').isEditable(),true);await page.locator('[name="branch"]').fill('master');await page.locator('#sync-setup-form [type="submit"]').click();await page.waitForSelector('#sync-setup-form',{state:'detached'});assert.match(await page.locator('.main-scroll').innerText(),/来自现有分支的笔记/);assert.deepEqual(errors,[]);await page.close();
 }
 console.log('桌面和手机原生字符串错误可见、国内仓库入口、用户名校验、令牌不落盘及指定现有分支下载通过；未使用真实私人仓库认证');
}finally{await browser?.close();server.kill('SIGTERM');}
